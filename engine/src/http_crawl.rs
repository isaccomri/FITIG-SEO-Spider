//! Bounded asynchronous HTTP jobs; SQLite and extraction remain on the caller.
use crate::{Result,CrawlProfile};
use rusqlite::{Connection,OptionalExtension};
use std::{collections::BTreeMap,sync::Arc,time::{Duration,Instant}};
use tokio::{runtime::Runtime,task::JoinSet,sync::{Mutex,OnceCell}};
use url::Url;
type RobotCell=Arc<OnceCell<std::result::Result<String,String>>>;
type Robots=Arc<Mutex<BTreeMap<String,RobotCell>>>;
type Rate=Arc<Mutex<Instant>>;
pub(crate) struct Fetched{pub url:String,pub depth:usize,pub status:u16,pub headers:BTreeMap<String,String>,pub body:Vec<u8>,pub error:String,pub blocked:bool,pub elapsed_ms:u128}
pub(crate) struct Scheduler{runtime:Runtime,jobs:JoinSet<Result<Fetched>>,client:reqwest::Client,authentication:Option<Arc<crate::authentication::Session>>,robots:Robots,scope:crate::scope::Scope,agent:String,concurrency:usize,next_dispatch:Instant,rate:Rate}
// With a nonzero interval, retain admission until response headers arrive so robots waits or connection setup cannot release a burst. Slow headers can conservatively reduce parallelism; zero-delay crawling keeps full concurrency.
async fn send(client:&reqwest::Client,url:&str,authentication:Option<&crate::authentication::Session>,rate:Option<(&Rate,Duration)>)->std::result::Result<reqwest::Response,reqwest::Error>{
 for attempt in 0..3{let mut permit=if let Some((gate,delay))=rate.filter(|(_,delay)|!delay.is_zero()){let next=gate.lock().await;tokio::time::sleep(next.saturating_duration_since(Instant::now())).await;Some((next,delay))}else{None};let dispatched=Instant::now();let mut request=client.get(url);if let Some(header)=authentication.and_then(|session|session.header_for(url)){request=request.header(reqwest::header::AUTHORIZATION,header)}let response=request.send().await;if let Some((next,delay))=permit.as_mut(){**next=dispatched+*delay;}drop(permit);match response{Ok(response)=>return Ok(response),Err(error)if attempt<2&&crate::transient_request(&error)=>tokio::time::sleep(Duration::from_millis(100*(1<<attempt))).await,Err(error)=>return Err(error)}}unreachable!()
}

#[cfg(test)]mod scope_tests {
 use super::*;
 use std::{io::{Read,Write},net::TcpListener,sync::atomic::{AtomicBool,Ordering}};
 struct Site {address:std::net::SocketAddr,stop:Arc<AtomicBool>,thread:Option<std::thread::JoinHandle<()>>,requests:Arc<std::sync::Mutex<Vec<String>>>}
 impl Site {
  fn new()->Self{let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();listener.set_nonblocking(true).unwrap();let stop=Arc::new(AtomicBool::new(false));let done=stop.clone();let requests=Arc::new(std::sync::Mutex::new(Vec::new()));let seen=requests.clone();let thread=std::thread::spawn(move||{while !done.load(Ordering::SeqCst){match listener.accept(){Ok((mut stream,_))=>{stream.set_nonblocking(false).unwrap();stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut bytes=Vec::new();let mut byte=[0];while bytes.len()<16384&&!bytes.ends_with(b"\r\n\r\n"){if stream.read(&mut byte).unwrap_or(0)!=1{break}bytes.push(byte[0])}let raw=String::from_utf8_lossy(&bytes);seen.lock().unwrap().push(raw.to_string());let request=raw.split_whitespace().nth(1).unwrap_or("");let host=raw.lines().find_map(|line|line.strip_prefix("host: ").or_else(||line.strip_prefix("Host: "))).unwrap_or("");let (status,body)=if request=="/robots.txt"{if host.starts_with("down."){("503 Service Unavailable","unavailable")}else if host.starts_with("blocked."){("200 OK","User-agent: *\nDisallow: /")}else{("200 OK","User-agent: *\nAllow: /")}}else{("200 OK","<title>Owned subdomain</title>")};let response=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());let _=stream.write_all(response.as_bytes());},Err(_)=>std::thread::sleep(Duration::from_millis(1))}}});Self{address,stop,thread:Some(thread),requests}}
 }
 impl Drop for Site{fn drop(&mut self){self.stop.store(true,Ordering::SeqCst);self.thread.take().unwrap().join().unwrap()}}
 #[test]fn explicit_subdomains_obey_per_origin_robots_and_internal_fail_closed(){
  let site=Site::new();let runtime=tokio::runtime::Runtime::new().unwrap();runtime.block_on(async{
   let mut builder=reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none());for host in ["www.example.test","shop.example.test","blocked.example.test","down.example.test","outside.test"]{builder=builder.resolve(host,site.address)}let client=builder.build().unwrap();let seed=Url::parse(&format!("http://www.example.test:{}/",site.address.port())).unwrap();let scope=crate::scope::Scope::new(&seed,"example.test").unwrap();let cache:Robots=Arc::new(Mutex::new(BTreeMap::new()));
   for host in ["www.example.test","shop.example.test","shop.example.test"]{let result=fetch(client.clone(),cache.clone(),scope.clone(),"FITIGBot".into(),format!("http://{host}:{}/page",site.address.port()),1).await.unwrap();assert_eq!(result.status,200);assert!(!result.blocked)}
   let blocked=fetch(client.clone(),cache.clone(),scope.clone(),"FITIGBot".into(),format!("http://blocked.example.test:{}/page",site.address.port()),1).await.unwrap();assert!(blocked.blocked);assert_eq!(blocked.status,0);
   assert!(fetch(client.clone(),cache.clone(),scope.clone(),"FITIGBot".into(),format!("http://down.example.test:{}/page",site.address.port()),1).await.is_err());
   let outside=crate::scope::Scope::new(&seed,"").unwrap();let observation=fetch(client,cache,outside,"FITIGBot".into(),format!("http://down.example.test:{}/page",site.address.port()),1).await.unwrap();assert!(observation.blocked);assert!(observation.error.starts_with("External robots unavailable"));
  });let requests=site.requests.lock().unwrap();assert_eq!(requests.iter().filter(|raw|raw.starts_with("GET /robots.txt")&&raw.contains("shop.example.test")).count(),1);assert!(!requests.iter().any(|raw|raw.starts_with("GET /page")&&(raw.contains("blocked.example.test")||raw.contains("down.example.test"))));
 }
}
async fn body(mut response:reqwest::Response,maximum:usize)->Result<Vec<u8>>{
 let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await?{let remaining=(maximum+1).saturating_sub(bytes.len());bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);if bytes.len()>maximum{break}}Ok(bytes)
}
async fn robots_text(client:&reqwest::Client,base:&Url,authentication:Option<&crate::authentication::Session>,rate:Option<(&Rate,Duration)>)->std::result::Result<String,String>{
 let response=send(client,base.join("/robots.txt").map_err(|e|e.to_string())?.as_str(),authentication,rate).await.map_err(|e|e.to_string())?;
 match response.status().as_u16(){200..=299=>{let bytes=body(response,1024*1024).await.map_err(|e|e.to_string())?;if bytes.len()>1024*1024{return Err("Robots exceeds 1 MiB".into())}Ok(String::from_utf8_lossy(&bytes).into_owned())},404=>Ok(String::new()),401|403=>Ok("User-agent: *\nDisallow: /".into()),407=>Err("Proxy authentication required (HTTP 407); check the configured proxy credentials".into()),_=>Err("Robots unavailable: crawl stopped conservatively".into())}
}
#[cfg(test)]
async fn fetch(client:reqwest::Client,cache:Robots,scope:crate::scope::Scope,agent:String,url:String,depth:usize)->Result<Fetched>{fetch_authenticated(client,cache,scope,agent,url,depth,None,None).await}
async fn fetch_authenticated(client:reqwest::Client,cache:Robots,scope:crate::scope::Scope,agent:String,url:String,depth:usize,authentication:Option<Arc<crate::authentication::Session>>,rate:Option<(Rate,Duration)>)->Result<Fetched>{
 let start=Instant::now();let parsed=Url::parse(&url)?;let origin=parsed.origin().ascii_serialization();
 let cell={cache.lock().await.entry(origin).or_insert_with(||Arc::new(OnceCell::new())).clone()};
 let gate=rate.as_ref().map(|(gate,delay)|(gate,*delay));let robots=cell.get_or_init(||robots_text(&client,&parsed,authentication.as_deref(),gate)).await;
 let mut result=Fetched{url:url.clone(),depth,status:0,headers:BTreeMap::new(),body:Vec::new(),error:String::new(),blocked:false,elapsed_ms:0};
 match robots{
  Err(error)if scope.contains(&parsed)=>return Err(error.clone().into()),
  Err(error)=>{result.blocked=true;result.error=format!("External robots unavailable: {error}")},
  Ok(text)=>{let target=match parsed.query(){Some(query)=>format!("{}?{}",parsed.path(),query),None=>parsed.path().into()};if !crate::robots_allowed_for(text,&target,&agent){result.blocked=true;result.error="Blocked by robots.txt".into()}}
 }
 if !result.blocked{match send(&client,&url,authentication.as_deref(),gate).await{Err(error)=>result.error=format!("{error:?}"),Ok(response)=>{
  result.status=response.status().as_u16();for(k,v)in response.headers().iter(){let value=v.to_str().unwrap_or("");if k=="x-robots-tag"{result.headers.entry(k.to_string()).and_modify(|old|{old.push('\n');old.push_str(value)}).or_insert_with(||value.into());}else{result.headers.insert(k.to_string(),value.into());}}
  match body(response,10*1024*1024).await{Ok(bytes)=>result.body=bytes,Err(error)=>result.error=format!("{error:?}")}
 }}}
 result.elapsed_ms=start.elapsed().as_millis();Ok(result)
}
impl Scheduler{
 pub fn new(seed:&Url,profile:&CrawlProfile,authentication:Option<crate::authentication::Session>,proxy_authentication:Option<crate::authentication::Session>,cookies:Option<Arc<crate::http_cookies::Jar>>)->Result<Self>{
  let runtime=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
  let mut builder=reqwest::Client::builder().user_agent(&profile.user_agent).redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(profile.request_timeout_secs));
  let local=seed.host_str().map(|host|host.eq_ignore_ascii_case("localhost")||host.trim_matches(['[',']']).parse::<std::net::IpAddr>().map(|ip|ip.is_loopback()).unwrap_or(false)).unwrap_or(false);if profile.proxy_mode=="manual"{builder=builder.no_proxy().proxy(crate::proxy_settings::build(profile,proxy_authentication.as_ref())?)}else if profile.proxy_mode=="direct"||local{builder=builder.no_proxy()}
  if let Some(jar)=cookies{builder=builder.cookie_provider(jar)}
  let client={let _entered=runtime.enter();builder.build()?};
  Ok(Self{runtime,jobs:JoinSet::new(),client,authentication:authentication.map(Arc::new),robots:Arc::new(Mutex::new(BTreeMap::new())),scope:crate::scope::Scope::new(seed,&profile.subdomain_root)?,agent:profile.user_agent.clone(),concurrency:profile.concurrency,next_dispatch:Instant::now(),rate:Arc::new(Mutex::new(Instant::now()))})
 }
 pub fn cancel(&mut self,connection:&Connection)->Result<()>{self.jobs.abort_all();self.runtime.block_on(async{while self.jobs.join_next().await.is_some(){}});connection.execute("UPDATE frontier SET state='pending' WHERE state='inflight'",[])?;Ok(())}
 pub fn next(&mut self,connection:&Connection,completed:usize,limit:usize,delay_ms:u64,stop:&std::path::Path)->Result<Option<Fetched>>{
  let result=self.runtime.block_on(async{
   loop{
    if stop.exists(){return Ok(None)}
    // A breadth-first layer barrier prevents a fast long path from assigning an inflated depth.
    while self.jobs.len()<self.concurrency&&(limit==0||completed+self.jobs.len()<limit)&&Instant::now()>=self.next_dispatch{
     let next:Option<(String,usize)>=connection.query_row("SELECT url,depth FROM frontier WHERE state='pending' AND depth=(SELECT MIN(depth) FROM frontier WHERE state IN ('pending','inflight')) ORDER BY rowid LIMIT 1",[],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
     let Some((url,depth))=next else{break};connection.execute("UPDATE frontier SET state='inflight' WHERE url=?1",[&url])?;
     self.jobs.spawn(fetch_authenticated(self.client.clone(),self.robots.clone(),self.scope.clone(),self.agent.clone(),url,depth,self.authentication.clone(),Some((self.rate.clone(),Duration::from_millis(delay_ms)))));self.next_dispatch=Instant::now()+Duration::from_millis(delay_ms);
    }
    if self.jobs.is_empty(){let pending:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM frontier WHERE state='pending')",[],|row|row.get(0))?;if !pending||limit!=0&&completed>=limit{return Ok(None)}tokio::time::sleep(Duration::from_millis(10)).await;continue}
    let wait=self.next_dispatch.saturating_duration_since(Instant::now());let wait=if wait.is_zero(){Duration::from_millis(100)}else{wait.min(Duration::from_millis(100))};
    if let Ok(Some(job))=tokio::time::timeout(wait,self.jobs.join_next()).await{if stop.exists(){return Ok(None)}return job.map_err(|error|->Box<dyn std::error::Error+Send+Sync>{error.into()})?.map(Some)}
   }
  });
  if result.is_err()||matches!(result,Ok(None)){self.cancel(connection)?}result
 }
}

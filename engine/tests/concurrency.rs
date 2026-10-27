use fitig_engine::{CrawlOptions,CrawlProfile,Page};
use std::{io::{Read,Write},net::{TcpListener,TcpStream},sync::{Arc,Mutex,atomic::{AtomicBool,AtomicUsize,Ordering}},time::{Duration,Instant},path::PathBuf};
struct Site{origin:String,done:Arc<AtomicBool>,server:Option<std::thread::JoinHandle<()>>,peak:Arc<AtomicUsize>,starts:Arc<Mutex<Vec<(String,Instant)>>>}
impl Site{
 fn new(handler:impl Fn(&str)->(u64,String)+Send+Sync+'static)->Self{
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let address=listener.local_addr().unwrap();
  let done=Arc::new(AtomicBool::new(false));let stop=done.clone();let active=Arc::new(AtomicUsize::new(0));let peak=Arc::new(AtomicUsize::new(0));let measured=peak.clone();let starts=Arc::new(Mutex::new(Vec::new()));let observed=starts.clone();let handler=Arc::new(handler);
  let server=std::thread::spawn(move||{let mut workers=Vec::new();while !stop.load(Ordering::SeqCst){match listener.accept(){Ok((mut stream,_))=>{let active=active.clone();let peak=measured.clone();let starts=observed.clone();let handler=handler.clone();workers.push(std::thread::spawn(move||{
   stream.set_nonblocking(false).unwrap();stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();let mut bytes=Vec::new();let mut one=[0];while bytes.len()<32768&&!bytes.ends_with(b"\r\n\r\n"){match stream.read(&mut one){Ok(1)=>bytes.push(one[0]),_=>return}}
   let request=String::from_utf8_lossy(&bytes);let path=request.split_whitespace().nth(1).unwrap_or("").to_string();starts.lock().unwrap().push((path.clone(),Instant::now()));
   let (wait,body)=if path=="/robots.txt"{(0,"User-agent: *\nAllow: /".into())}else{let n=active.fetch_add(1,Ordering::SeqCst)+1;peak.fetch_max(n,Ordering::SeqCst);handler(&path)};
   if body=="SLOW_BODY"{let payload="<p>late</p>";let _=write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",payload.len());std::thread::sleep(Duration::from_millis(wait));let _=stream.write_all(payload.as_bytes());active.fetch_sub(1,Ordering::SeqCst);return}std::thread::sleep(Duration::from_millis(wait));if body=="RESET"{socket2::SockRef::from(&stream).set_linger(Some(Duration::ZERO)).unwrap();if path!="/robots.txt"{active.fetch_sub(1,Ordering::SeqCst);}return}let response=format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());let _=stream.write_all(response.as_bytes());if path!="/robots.txt"{active.fetch_sub(1,Ordering::SeqCst);}
  }));},Err(_)=>std::thread::sleep(Duration::from_millis(1))}}for worker in workers{worker.join().unwrap();}});
  Self{origin:format!("http://{address}"),done,server:Some(server),peak,starts}
 }
 fn pages(&self)->Vec<(String,Instant)>{self.starts.lock().unwrap().iter().filter(|(path,_)|path!="/robots.txt").cloned().collect()}
}
impl Drop for Site{fn drop(&mut self){self.done.store(true,Ordering::SeqCst);let _=TcpStream::connect(self.origin.trim_start_matches("http://"));self.server.take().unwrap().join().unwrap();}}
static PROJECT_COUNTER:AtomicUsize=AtomicUsize::new(0);
struct Project(PathBuf);impl Project{fn new()->Self{let p=std::env::temp_dir().join(format!("fitig-concurrent-{}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),PROJECT_COUNTER.fetch_add(1,Ordering::SeqCst)));std::fs::create_dir(&p).unwrap();Self(p)}fn db(&self)->String{self.0.join("crawl.sqlite").to_string_lossy().into_owned()}}
impl Drop for Project{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn options(site:&Site,project:&Project,limit:usize)->CrawlOptions{CrawlOptions{url:format!("{}/",site.origin),db:project.db(),limit,delay_ms:0,resume:false}}
#[test]fn rewriting_collapses_discovered_requests_preserves_seed_and_original_links(){
 let site=Site::new(|path|if path.starts_with("/start"){(0,"<a href='/old/12?utm_source=a'>אחד</a><a href='/old/12?utm_source=b'>שניים</a><img src='/image.png?utm_source=c' alt='תמונה'>".into())}else{(0,"<title>Rewritten request</title>".into())});let project=Project::new();let profile=CrawlProfile{url_rewriting:fitig_engine::rewriting::Options{enabled:true,remove_parameters:vec!["utm_source".into()],rules:vec![fitig_engine::rewriting::Rule{pattern:"/old/([0-9]+)".into(),replacement:"/new/$1".into()}],..Default::default()},..Default::default()};
 let seed=format!("{}/start?utm_source=seed",site.origin);let pages=fitig_engine::crawl_configured(CrawlOptions{url:seed.clone(),..options(&site,&project,0)},None,profile.clone(),|_|{}).unwrap();assert_eq!(pages.len(),3);let requests=site.pages();assert!(requests.iter().any(|(url,_)|url=="/start?utm_source=seed"));assert_eq!(requests.iter().filter(|(url,_)|url=="/new/12").count(),1);assert!(!requests.iter().any(|(url,_)|url.starts_with("/old/")));let page=pages.iter().find(|p|p.url==seed).unwrap();assert!(page.links.iter().all(|link|link.url.contains("/old/12?utm_source=")));assert_eq!(page.rewrite_count,Some(3));let before=std::fs::read(project.db()).unwrap();let report=fitig_engine::rewriting::load(&project.db(),&seed).unwrap().unwrap();assert_eq!((report.changed,report.errors,report.entries.len()),(3,0,3));assert!(report.entries.iter().all(|entry|entry.disposition=="Eligible for queue"));assert_eq!(fitig_engine::project_profile(&project.db()).unwrap(),profile);assert_eq!(std::fs::read(project.db()).unwrap(),before);
}
#[test]fn rewriting_respects_result_scope_filters_and_invalid_target_errors(){
 let site=Site::new(|_|(0,"<a href='/off'>off</a><a href='/bad'>bad</a><a href='/exclude'>exclude</a><a href='/ok'>ok</a>".into()));let project=Project::new();let profile=CrawlProfile{exclude_patterns:vec!["/filtered$".into()],url_rewriting:fitig_engine::rewriting::Options{enabled:true,rules:vec![fitig_engine::rewriting::Rule{pattern:"^http://[^/]+/off$".into(),replacement:"https://outside.test/page".into()},fitig_engine::rewriting::Rule{pattern:"^http://[^/]+/bad$".into(),replacement:"javascript:alert(1)".into()},fitig_engine::rewriting::Rule{pattern:"/exclude$".into(),replacement:"/filtered".into()}],..Default::default()},..Default::default()};let pages=fitig_engine::crawl_configured(options(&site,&project,1),None,profile,|_|{}).unwrap();let report=fitig_engine::rewriting::load(&project.db(),&pages[0].url).unwrap().unwrap();assert_eq!((report.changed,report.errors),(2,1));assert!(report.entries.iter().any(|entry|entry.disposition.starts_with("Outside scope")));assert!(report.entries.iter().any(|entry|entry.disposition.starts_with("URL filter")));assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),1);assert_eq!(site.pages().len(),1);
}
#[test]fn rewriting_does_not_change_manual_list_entries_or_create_invalid_projects(){
 let site=Site::new(|_|(0,"<a href='/next?sid=1'>next</a>".into()));let project=Project::new();let profile=CrawlProfile{url_rewriting:fitig_engine::rewriting::Options{enabled:true,remove_all_parameters:true,..Default::default()},..Default::default()};let urls=vec![format!("{}/one?sid=manual",site.origin),format!("{}/two?sid=manual",site.origin)];let pages=fitig_engine::crawl_configured(options(&site,&project,0),Some(urls.clone()),profile.clone(),|_|{}).unwrap();assert_eq!(pages.len(),2);assert!(pages.iter().all(|page|urls.contains(&page.url)));assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),0);let snapshot=std::fs::read(project.db()).unwrap();assert!(fitig_engine::crawl_configured(CrawlOptions{resume:true,..options(&site,&project,0)},Some(urls),CrawlProfile::default(),|_|{}).is_err());assert_eq!(std::fs::read(project.db()).unwrap(),snapshot);let invalid=Project::new();let invalid_profile=CrawlProfile{url_rewriting:fitig_engine::rewriting::Options{rules:vec![fitig_engine::rewriting::Rule{pattern:"[".into(),replacement:String::new()}],..Default::default()},..Default::default()};assert!(fitig_engine::crawl_configured(options(&site,&invalid,1),None,invalid_profile,|_|{}).is_err());assert!(!std::path::Path::new(&invalid.db()).exists());
}
#[test]fn bounded_parallel_list_obeys_exact_limit_and_fetches_robots_once(){
 let site=Site::new(|_|(160,"<title>Concurrent</title><body>שלום</body>".into()));let project=Project::new();let profile=CrawlProfile{concurrency:3,..Default::default()};
 let pages=fitig_engine::crawl_configured(options(&site,&project,5),Some((0..12).map(|i|format!("{}/p{i}",site.origin)).collect()),profile,|_|{}).unwrap();
 assert_eq!(pages.len(),5);assert_eq!(site.pages().len(),5);assert_eq!(site.peak.load(Ordering::SeqCst),3);assert_eq!(site.starts.lock().unwrap().iter().filter(|(p,_)|p=="/robots.txt").count(),1);assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),7);
}
#[test]fn parallel_breadth_first_barrier_preserves_shortest_depth(){
 let site=Site::new(|path|match path{"/"=>(0,"<a href='/slow'>slow</a><a href='/fast'>fast</a>".into()),"/slow"=>(200,"<a href='/target'>short path</a>".into()),"/fast"=>(0,"<a href='/middle'>long path</a>".into()),"/middle"=>(0,"<a href='/target'>target</a>".into()),_=>(0,"<title>Target</title>".into())});let project=Project::new();
 let pages=fitig_engine::crawl(options(&site,&project,0),|_|{}).unwrap();let target=pages.iter().find(|p|p.url.ends_with("/target")).unwrap();assert_eq!(target.depth,2);assert_eq!(pages.len(),5);
 let starts=site.pages();let slow=starts.iter().find(|(p,_)|p=="/slow").unwrap().1;let middle=starts.iter().find(|(p,_)|p=="/middle").unwrap().1;assert!(middle.duration_since(slow)>=Duration::from_millis(180));
}
#[test]fn stop_resets_uncommitted_jobs_and_resume_completes_each_url_once(){
 let site=Site::new(|_|(120,"<title>Saved</title>".into()));let project=Project::new();let urls:Vec<_>=(0..10).map(|i|format!("{}/p{i}",site.origin)).collect();let profile=CrawlProfile{concurrency:3,..Default::default()};let opts=options(&site,&project,0);let stop=format!("{}.stop",project.db());
 let partial=fitig_engine::crawl_configured(opts.clone(),Some(urls.clone()),profile.clone(),|p|{if p.completed==2{std::fs::write(&stop,b"stop").unwrap();}}).unwrap();assert_eq!(partial.len(),2);assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),8);
 {let c=rusqlite::Connection::open(project.db()).unwrap();assert_eq!(c.query_row("SELECT COUNT(*) FROM frontier WHERE state='inflight'",[],|r|r.get::<_,usize>(0)).unwrap(),0);}
 let before=site.pages().len();std::thread::sleep(Duration::from_millis(150));assert_eq!(site.pages().len(),before);std::fs::remove_file(stop).unwrap();
 let resumed=fitig_engine::crawl_configured(CrawlOptions{resume:true,..opts},Some(urls),profile,|_|{}).unwrap();assert_eq!(resumed.len(),10);assert!(resumed.iter().all(|p|p.status==200));assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),0);
}
#[test]fn global_dispatch_delay_is_not_multiplied_by_worker_count(){
 let site=Site::new(|_|(90,"<title>Paced</title>".into()));let project=Project::new();let mut opts=options(&site,&project,0);opts.delay_ms=50;
 fitig_engine::crawl_configured(opts,Some((0..5).map(|i|format!("{}/p{i}",site.origin)).collect()),CrawlProfile::default(),|_|{}).unwrap();let starts=site.pages();assert_eq!(starts.len(),5);for pair in starts.windows(2){assert!(pair[1].1.duration_since(pair[0].1)>=Duration::from_millis(40),"Observed server gap {:?}: {} -> {}",pair[1].1.duration_since(pair[0].1),pair[0].0,pair[1].0);}assert!(site.peak.load(Ordering::SeqCst)<=5);
}
#[test]fn invalid_concurrency_does_not_create_project(){let site=Site::new(|_|panic!("No HTTP expected"));for n in [0,33]{let project=Project::new();assert!(fitig_engine::crawl_configured(options(&site,&project,0),None,CrawlProfile{concurrency:n,..Default::default()},|_|{}).is_err());assert!(!std::path::Path::new(&project.db()).exists());}assert!(site.starts.lock().unwrap().is_empty());}
#[test]fn killed_owned_cli_recovers_durable_inflight_frontier(){
 let site=Site::new(|path|if path=="/"{(0,(0..12).map(|i|format!("<a href='/p{i}'>page</a>")).collect())}else{(250,"<title>Recovered</title>".into())});let project=Project::new();
 let mut child=std::process::Command::new(env!("CARGO_BIN_EXE_fitig-engine")).args(["crawl","--url",&format!("{}/",site.origin),"--db",&project.db(),"--delay-ms","0"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
 let start=Instant::now();let mut inflight=0;while start.elapsed()<Duration::from_secs(10){if let Ok(c)=rusqlite::Connection::open_with_flags(project.db(),rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY){inflight=c.query_row("SELECT COUNT(*) FROM frontier WHERE state='inflight'",[],|r|r.get::<_,usize>(0)).unwrap_or(0);if inflight>=3{break}}std::thread::sleep(Duration::from_millis(10));}
 child.kill().unwrap();child.wait().unwrap();assert!(inflight>=3,"Owned CLI never reached concurrent requests");
 let pages:Vec<Page>=fitig_engine::crawl(CrawlOptions{resume:true,..options(&site,&project,0)},|_|{}).unwrap();assert_eq!(pages.len(),13);assert!(pages.iter().all(|p|p.status==200));assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),0);
}

#[test]fn asynchronous_get_retries_real_connection_resets_with_a_three_attempt_bound(){
 for failures in [2,3]{
  let attempts=Arc::new(AtomicUsize::new(0));let counter=attempts.clone();let site=Site::new(move |_|{let n=counter.fetch_add(1,Ordering::SeqCst);(0,if n<failures{"RESET".into()}else{"<title>Recovered reset</title>".into()})});let project=Project::new();
  let pages=fitig_engine::crawl(options(&site,&project,1),|_|{}).unwrap();assert_eq!(attempts.load(Ordering::SeqCst),3);assert_eq!(pages.len(),1);
  if failures==2{assert_eq!(pages[0].status,200);assert_eq!(pages[0].title,"Recovered reset")}else{assert_eq!(pages[0].status,0);assert!(pages[0].issues.iter().any(|issue|issue=="Request failed"));}
 }
}
#[test]fn received_response_hash_persists_exact_bytes_not_normalized_content(){
 use sha2::{Digest,Sha256};
 let body="<body><main>שלום עולם</main><!-- exact bytes --></body>";
 let site=Site::new(move|path|(0,if path=="/"{format!("<a href='/a'>a</a><a href='/b'>b</a><a href='/c'>c</a>")}else if path=="/c"{body.replace("exact bytes","different comment")}else{body.into()}));
 let project=Project::new();let pages=fitig_engine::crawl_configured(options(&site,&project,0),None,CrawlProfile::default(),|_|{}).unwrap();
 let a=pages.iter().find(|p|p.url.ends_with("/a")).unwrap();let b=pages.iter().find(|p|p.url.ends_with("/b")).unwrap();let c=pages.iter().find(|p|p.url.ends_with("/c")).unwrap();
 assert_eq!(a.response_hash,Some(format!("{:x}",Sha256::digest(body.as_bytes()))));assert_eq!(a.response_hash,b.response_hash);assert_ne!(a.response_hash,c.response_hash);assert_eq!(a.content_hash,c.content_hash);
 let before=std::fs::read(project.db()).unwrap();let saved=fitig_engine::load_pages(&project.db()).unwrap();assert_eq!(saved.iter().find(|p|p.url==a.url).unwrap().response_hash,a.response_hash);assert_eq!(std::fs::read(project.db()).unwrap(),before);
 let report=fitig_engine::analyze(&project.db()).unwrap();let groups=report["duplicate_response_bodies"].as_object().unwrap();assert_eq!(groups.len(),1);assert_eq!(groups.values().next().unwrap().as_array().unwrap().len(),2);

}

#[test]fn configured_timeout_bounds_headers_and_body_retries_without_losing_fast_pages(){
 for body in [false,true]{
  let site=Site::new(move|path|if path=="/slow"{(1800,if body{"SLOW_BODY".into()}else{"<p>late headers</p>".into()})}else{(0,"<title>Fast</title>".into())});let project=Project::new();let profile=CrawlProfile{request_timeout_secs:1,..Default::default()};let urls=vec![format!("{}/fast",site.origin),format!("{}/slow",site.origin)];let start=Instant::now();
  let pages=fitig_engine::crawl_configured(options(&site,&project,0),Some(urls),profile.clone(),|_|{}).unwrap();assert!(start.elapsed()<Duration::from_secs(10));assert_eq!(pages.len(),2);assert_eq!(pages.iter().find(|p|p.url.ends_with("/fast")).unwrap().status,200);let slow=pages.iter().find(|p|p.url.ends_with("/slow")).unwrap();assert_eq!(slow.status,0);assert!(slow.error.to_ascii_lowercase().contains("timedout")||slow.error.to_ascii_lowercase().contains("timed out"));assert!(slow.response_hash.is_none());assert_eq!(site.pages().iter().filter(|(path,_)|path=="/slow").count(),if body{1}else{3});assert_eq!(fitig_engine::project_profile(&project.db()).unwrap(),profile);assert_eq!(fitig_engine::pending_count(&project.db()).unwrap(),0);
 }
}
#[test]fn invalid_timeout_is_rejected_before_project_creation_and_resume_keeps_profile(){
 let site=Site::new(|_|(0,"<title>Fast</title>".into()));
 for value in [0,301]{let project=Project::new();let result=fitig_engine::crawl_configured(options(&site,&project,0),None,CrawlProfile{request_timeout_secs:value,..Default::default()},|_|{});assert!(result.unwrap_err().to_string().contains("timeout"));assert!(!std::path::Path::new(&project.db()).exists())}
 assert!(site.pages().is_empty());let project=Project::new();let profile=CrawlProfile{request_timeout_secs:3,..Default::default()};fitig_engine::crawl_configured(options(&site,&project,1),None,profile.clone(),|_|{}).unwrap();let before=std::fs::read(project.db()).unwrap();let mut resume=options(&site,&project,0);resume.resume=true;let result=fitig_engine::crawl_configured(resume,None,CrawlProfile{request_timeout_secs:4,..Default::default()},|_|{});assert!(result.is_err());assert_eq!(std::fs::read(project.db()).unwrap(),before);assert_eq!(fitig_engine::project_profile(&project.db()).unwrap(),profile);
}

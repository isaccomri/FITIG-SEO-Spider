//! Explicit sitemap retrieval, separate from the crawl frontier.
use crate::Result;
use std::{collections::{HashMap,HashSet,VecDeque},io::Read,time::{Duration,Instant}};
use serde::Serialize;
use url::Url;
const NS:&str="http://www.sitemaps.org/schemas/sitemap/0.9";
const FILE_LIMIT:usize=50*1024*1024;
#[derive(Debug,Serialize)]
pub struct Inventory{pub locations:Vec<String>,pub files:Vec<String>,pub missing:Vec<String>}
fn location(raw:&str)->Result<Url>{let url=Url::parse(raw.trim())?;if !matches!(url.scheme(),"http"|"https")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.fragment().is_some(){return Err("Sitemap URLs require absolute HTTP(S), no credentials or fragments".into())}Ok(url)}
fn bounded(mut reader:impl Read,limit:usize)->Result<Vec<u8>>{let mut bytes=Vec::new();reader.by_ref().take(limit as u64+1).read_to_end(&mut bytes)?;if bytes.len()>limit{return Err("Sitemap exceeds size limit".into())}Ok(bytes)}
fn unpack(bytes:Vec<u8>,limit:usize)->Result<Vec<u8>>{if bytes.starts_with(&[0x1f,0x8b]){bounded(flate2::read::MultiGzDecoder::new(bytes.as_slice()),limit)}else{if bytes.len()>limit{return Err("Sitemap exceeds size limit".into())}Ok(bytes)}}
pub fn parse(xml:&str)->Result<(bool,Vec<String>)>{
 if xml.contains("<!DOCTYPE")||xml.contains("<!ENTITY"){return Err("Sitemap DTD and entity declarations are not supported".into())}
 let package=sxd_document::parser::parse(xml).map_err(|_|"Invalid sitemap XML")?;
 let document=package.as_document();let root=document.root().children().into_iter().find_map(|child|child.element()).ok_or("Missing sitemap root")?;
 let index=match(root.name().namespace_uri(),root.name().local_part()){(Some(NS),"urlset")=>false,(Some(NS),"sitemapindex")=>true,_=>return Err("Expected standard namespace sitemap urlset or sitemapindex".into())};
 let mut locations=Vec::new();
 for entry in root.children().into_iter().filter_map(|child|child.element()).filter(|element|element.name().namespace_uri()==Some(NS)&&element.name().local_part()==if index{"sitemap"}else{"url"}){
  let locs:Vec<_>=entry.children().into_iter().filter_map(|child|child.element()).filter(|element|element.name().namespace_uri()==Some(NS)&&element.name().local_part()=="loc").collect();
  if locs.len()!=1{return Err("Each sitemap entry requires exactly one loc".into())}
  let mut text=String::new();for child in locs[0].children(){if child.element().is_some(){return Err("Sitemap loc must contain text only".into())}if let Some(value)=child.text(){text.push_str(value.text())}}
  locations.push(location(&text)?.to_string());if locations.len()>50000{return Err("Sitemap exceeds 50,000 entries".into())}
 }
 Ok((index,locations))
}
fn fetch_limited(client:&reqwest::blocking::Client,mut url:Url,origin:&url::Origin,wire_limit:usize,expanded_limit:usize)->Result<(Url,Vec<u8>)>{
 for _ in 0..=5{
  let response=client.get(url.as_str()).send()?;
  if response.status().is_redirection(){let next=response.headers().get("location").ok_or("Sitemap redirect missing Location")?.to_str()?;url=location(url.join(next)?.as_str())?;if &url.origin()!=origin{return Err("Cross-origin sitemap redirect is not followed".into())}continue}
  if !response.status().is_success(){return Err(format!("Sitemap request returned HTTP {}",response.status().as_u16()).into())}
  let content_encoding=response.headers().get("content-encoding").and_then(|value|value.to_str().ok()).unwrap_or("").to_ascii_lowercase();
  if !content_encoding.is_empty()&&!matches!(content_encoding.as_str(),"gzip"|"identity"){return Err("Unsupported sitemap HTTP content encoding".into())}
  let mut bytes=bounded(response,wire_limit)?;
  if content_encoding=="gzip"{if !bytes.starts_with(&[0x1f,0x8b]){return Err("Invalid HTTP gzip response".into())}bytes=unpack(bytes,expanded_limit)?}
  return Ok((url,unpack(bytes,expanded_limit)?))
 }
 Err("Sitemap redirect limit exceeded".into())
}
fn fetch(client:&reqwest::blocking::Client,url:Url,origin:&url::Origin)->Result<(Url,Vec<u8>)>{fetch_limited(client,url,origin,10*1024*1024,FILE_LIMIT)}
#[derive(Debug,Serialize)]pub struct Discovered{pub url:String,pub same_origin:bool}
#[derive(Debug,Serialize)]pub struct Discovery{pub robots_url:String,pub effective_url:String,pub sitemaps:Vec<Discovered>,pub warnings:Vec<String>}
pub fn parse_robots_sitemaps(text:&str,seed:&str)->Result<(Vec<Discovered>,Vec<String>)>{
 let prefix=text.chars().take(1024).collect::<String>().to_ascii_lowercase();if prefix.contains("<html")||prefix.contains("<!doctype html"){return Err("Robots endpoint returned HTML instead of robots.txt text".into())}
 let origin=location(seed)?.origin();let mut found=Vec::new();let mut warnings=Vec::new();let mut seen=HashSet::new();
 for (line_number,line) in text.trim_start_matches('\u{feff}').lines().enumerate(){let line=line.split('#').next().unwrap_or("").trim();let Some((key,value))=line.split_once(':')else{continue};if !key.trim().eq_ignore_ascii_case("sitemap"){continue}
  match location(value){Ok(url)=>{if seen.insert(url.to_string()){if found.len()>=100{return Err("Robots discovery exceeds 100 distinct sitemap directives".into())}found.push(Discovered{same_origin:url.origin()==origin,url:url.to_string()})}},Err(_)=>warnings.push(format!("Invalid absolute sitemap URL on robots.txt line {}",line_number+1))}
 }
 Ok((found,warnings))
}
pub fn discover(seed:&str,user_agent:&str)->Result<Discovery>{
 let seed=location(seed)?;let mut robots=seed.clone();robots.set_path("/robots.txt");robots.set_query(None);let robots_url=robots.to_string();
 let client=reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(20)).user_agent(user_agent).build()?;
 let(effective,bytes)=fetch_limited(&client,robots,&seed.origin(),1024*1024,1024*1024)?;let text=std::str::from_utf8(&bytes).map_err(|_|"Robots discovery requires UTF-8 text")?;let(sitemaps,warnings)=parse_robots_sitemaps(text,seed.as_str())?;
 Ok(Discovery{robots_url,effective_url:effective.to_string(),sitemaps,warnings})
}
pub fn retrieve(raw:&str,user_agent:&str)->Result<Inventory>{retrieve_many(&[raw.to_string()],user_agent)}
pub fn retrieve_many(raws:&[String],user_agent:&str)->Result<Inventory>{
 if raws.is_empty()||raws.len()>100{return Err("Choose 1–100 sitemap root URLs".into())}let roots:Vec<Url>=raws.iter().map(|raw|location(raw)).collect::<Result<_>>()?;let origin=roots[0].origin();if roots.iter().any(|url|url.origin()!=origin){return Err("Retrieve sitemap roots from one origin at a time".into())}
 let client=reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(20)).user_agent(user_agent).build()?;
 let start=Instant::now();let mut scheduled=HashSet::new();let mut queue=VecDeque::new();for root in roots{if scheduled.insert(root.to_string()){queue.push_back((root,Vec::<String>::new()))}}let mut seen=HashSet::new();let mut graph=HashMap::<String,Vec<String>>::new();let mut locations_seen=HashSet::new();let mut inventory=Inventory{locations:Vec::new(),files:Vec::new(),missing:Vec::new()};let mut total=0usize;
 while let Some((url,parents))=queue.pop_front(){
  if parents.contains(&url.to_string()){return Err("Sitemap index cycle detected".into())}
  if !seen.insert(url.to_string()){continue}
  if inventory.files.len()>=100||parents.len()>10||start.elapsed()>Duration::from_secs(120){return Err("Sitemap retrieval limit reached (100 files, depth 10, 120 seconds); inventory not saved".into())}
  if !inventory.files.is_empty(){std::thread::sleep(Duration::from_millis(100))}
  let (effective,bytes)=fetch(&client,url.clone(),&origin)?;total+=bytes.len();if total>100*1024*1024{return Err("Sitemap retrieval exceeds 100 MiB total".into())}
  let xml=std::str::from_utf8(&bytes).map_err(|_|"Sitemaps must use UTF-8")?.trim_start_matches('\u{feff}');let (index,locations)=parse(xml)?;
  inventory.files.push(effective.to_string());if effective!=url{graph.entry(url.to_string()).or_default().push(effective.to_string());}
  if index{let mut next=parents;next.push(url.to_string());if effective!=url{next.push(effective.to_string())}for child in locations{let child=location(&child)?;graph.entry(effective.to_string()).or_default().push(child.to_string());if child.origin()!=origin{return Err("Cross-origin child sitemap requires a separate explicit retrieval".into())}if next.contains(&child.to_string()){return Err("Sitemap index cycle detected".into())}if scheduled.insert(child.to_string()){if scheduled.len()>100{return Err("Sitemap index exceeds 100 distinct files; inventory not saved".into())}queue.push_back((child,next.clone()))}}}
  else{for value in locations{if locations_seen.insert(value.clone()){inventory.locations.push(value)}}if inventory.locations.len()>250000{return Err("Sitemap retrieval exceeds 250,000 entries".into())}}
 }
 if has_cycle(&graph){return Err("Sitemap index cycle detected".into())}Ok(inventory)
}
fn has_cycle(graph:&HashMap<String,Vec<String>>)->bool{
 fn visit(node:&str,graph:&HashMap<String,Vec<String>>,active:&mut HashSet<String>,done:&mut HashSet<String>)->bool{if active.contains(node){return true}if done.contains(node){return false}active.insert(node.into());if graph.get(node).is_some_and(|children|children.iter().any(|child|visit(child,graph,active,done))){return true}active.remove(node);done.insert(node.into());false}
 let mut active=HashSet::new();let mut done=HashSet::new();graph.keys().any(|node|visit(node,graph,&mut active,&mut done))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn html_challenge_is_not_reported_as_an_empty_robots_file(){for text in ["<!DOCTYPE html><html><title>One moment, please...</title>","\u{feff} <HTML lang='en'><body>Sitemap: https://e.test/x.xml"]{assert!(parse_robots_sitemaps(text,"https://e.test/").unwrap_err().to_string().contains("returned HTML"));}}
 #[test]fn robots_directives_are_global_case_insensitive_and_deduplicated(){let(found,warnings)=parse_robots_sitemaps("\u{feff}User-agent: blocked\nDisallow: /\n SITEMAP : https://e.test/מפה.xml # note\nSitemap: https://e.test/%D7%9E%D7%A4%D7%94.xml\nUser-agent: *\nSitemap: https://other.test/map.xml\nSitemap: /relative.xml\nSitemap: file:///secret\nSitemap: https://user:secret@e.test/x","https://e.test/path").unwrap();assert_eq!(found.len(),2);assert!(found[0].same_origin);assert!(!found[1].same_origin);assert_eq!(warnings.len(),3);assert!(warnings.iter().all(|warning|!warning.contains("secret")));}
 #[test]fn discovery_has_no_guessed_default_and_bounds_distinct_directives(){assert!(parse_robots_sitemaps("User-agent: *\nDisallow: /sitemap.xml","https://e.test/").unwrap().0.is_empty());let many=(0..101).map(|i|format!("Sitemap: https://e.test/{i}.xml\n")).collect::<String>();assert!(parse_robots_sitemaps(&many,"https://e.test/").is_err());assert!(retrieve_many(&[],"FITIG test").is_err());assert!(retrieve_many(&["https://e.test/x".into(),"https://other.test/x".into()],"FITIG test").is_err());}
 #[test]fn graph_cycle_detection_covers_independently_queued_roots_and_shared_children(){let mut graph=HashMap::from([("a".into(),vec!["b".into()]),("b".into(),vec!["c".into()]),("d".into(),vec!["c".into()])]);assert!(!has_cycle(&graph));graph.insert("c".into(),vec!["a".into()]);assert!(has_cycle(&graph));}
 #[test]fn namespace_hebrew_entities_and_invalid_inputs(){
  let xml=format!("<s:urlset xmlns:s='{NS}'><s:url><s:loc>https://e.test/שלום?a=1&amp;b=2</s:loc></s:url></s:urlset>");let(index,urls)=parse(&xml).unwrap();assert!(!index);assert_eq!(urls[0],"https://e.test/%D7%A9%D7%9C%D7%95%D7%9D?a=1&b=2");
  for xml in ["<!DOCTYPE x><urlset/>","<urlset/>",&format!("<urlset xmlns='{NS}'><url><loc>file:///x</loc></url></urlset>"),&format!("<urlset xmlns='{NS}'><url><loc>https://e.test/</loc><loc>https://e.test/x</loc></url></urlset>")]{assert!(parse(xml).is_err())}
 }
 #[test]fn gzip_and_expansion_limit(){use std::io::Write;let mut writer=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::default());writer.write_all("שלום".repeat(100).as_bytes()).unwrap();let bytes=writer.finish().unwrap();assert_eq!(unpack(bytes.clone(),1024).unwrap(),"שלום".repeat(100).as_bytes());assert!(unpack(bytes,16).is_err());assert!(bounded(&b"12345"[..],4).is_err());}
 #[test]fn rejects_unrequested_protocols_and_credentials(){for value in ["file:///x","https://user:pass@e.test/x","https://e.test/x#fragment","/relative"]{assert!(location(value).is_err())}}
}

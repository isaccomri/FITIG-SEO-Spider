//! Recorded-field comparison: explicitly distinguishes current and baseline inventories.
use crate::{Page,Result};
use serde::Serialize;
use serde_json::{json,Value};
use std::collections::{BTreeMap,BTreeSet};
#[derive(Debug,Serialize)]pub struct Row{pub url:String,pub change:String,pub fields:Vec<String>,pub baseline:Option<BTreeMap<String,Value>>,pub current:Option<BTreeMap<String,Value>>}
#[derive(Debug,Serialize)]pub struct Report{pub current_path:String,pub baseline_path:String,pub current_seed:String,pub baseline_seed:String,pub counts:BTreeMap<String,usize>,pub rows:Vec<Row>,pub note:String}
fn sorted<T:Serialize>(items:&[T])->Value{let mut values:Vec<_>=items.iter().map(|item|serde_json::to_value(item).unwrap()).collect();values.sort_by_cached_key(|value|value.to_string());json!(values)}
fn summaries(pages:&[Page])->BTreeMap<String,BTreeMap<String,Value>>{
 let mut incoming:BTreeMap<&str,BTreeSet<&str>>=BTreeMap::new();
 for page in pages{for link in page.links.iter().filter(|link|link.kind=="anchor"){incoming.entry(&link.url).or_default().insert(&page.url);}}
 pages.iter().map(|page|{
  let mut values=BTreeMap::new();macro_rules! field{($($name:ident),*)=>{$(values.insert(stringify!($name).into(),json!(page.$name));)*}}
  field!(status,content_type,title,description,canonical,directives,word_count,content_hash,response_hash,depth,redirect,charset,rendered_url,error,headings,internal);
  values.insert("hreflang".into(),sorted(&page.hreflang));values.insert("links".into(),sorted(&page.links));values.insert("images".into(),sorted(&page.images));values.insert("json_ld".into(),sorted(&page.json_ld));values.insert("issues".into(),sorted(&page.issues));values.insert("custom".into(),sorted(&page.custom));
  values.insert("unique_inlinks".into(),json!(incoming.get(page.url.as_str()).map_or(0,|links|links.len())));
  values.insert("unique_outlinks".into(),json!(page.links.iter().filter(|link|link.kind=="anchor").map(|link|&link.url).collect::<BTreeSet<_>>().len()));
  (page.url.clone(),values)
 }).collect()
}
pub fn analyze(current:&[Page],baseline:&[Page])->Vec<Row>{
 let current=summaries(current);let baseline=summaries(baseline);let urls:BTreeSet<_>=current.keys().chain(baseline.keys()).cloned().collect();
 urls.into_iter().map(|url|{
  let before=baseline.get(&url);let after=current.get(&url);let fields=match(before,after){(Some(a),Some(b))=>a.keys().filter(|key|a.get(*key)!=b.get(*key)&&(*key!="response_hash"||a.get(*key).is_some_and(Value::is_string)&&b.get(*key).is_some_and(Value::is_string))).cloned().collect::<Vec<_>>(),_=>Vec::new()};
  let change=match(before,after){(None,Some(_))=>"Added",(Some(_),None)=>"Removed",_ if !fields.is_empty()=>"Changed",_=>"Unchanged"}.to_string();
  Row{url,change,fields,baseline:before.cloned(),current:after.cloned()}
 }).collect()
}
pub fn projects(current:&str,baseline:&str)->Result<Report>{
 let rows=analyze(&crate::load_pages(current)?,&crate::load_pages(baseline)?);let mut counts:BTreeMap<String,usize>=["Added","Removed","Changed","Unchanged"].into_iter().map(|name|(name.into(),0)).collect();for row in &rows{*counts.get_mut(&row.change).unwrap()+=1;}
 Ok(Report{current_path:current.into(),baseline_path:baseline.into(),current_seed:crate::project_seed(current)?,baseline_seed:crate::project_seed(baseline)?,counts,rows,note:"Comparison of recorded observations. Added/Removed describe crawl inventories, not proof that a URL was created/deleted on the website. Missing observations remain unknown. Volatile response timing, transfer size and response headers are excluded; list order in links/images/hreflang/issues/custom results is ignored. Changes in extraction settings or versions can also produce differences.".into()})
}
#[cfg(test)]mod tests{
 use super::*;use crate::Link;
 fn page(url:&str,title:&str)->Page{Page{url:format!("https://e.test/{url}"),title:title.into(),status:200,content_type:"text/html".into(),..Default::default()}}
 #[test]fn current_baseline_orientation_missing_values_and_fields(){
  let old=vec![page("same","unchanged"),page("changed","ישן"),page("removed","removed")];let new=vec![page("same","unchanged"),Page{status:404,word_count:9,headings:vec![vec!["כותרת".into()]],..page("changed","חדש")},page("added","added")];let rows=analyze(&new,&old);
  let get=|suffix:&str|rows.iter().find(|row|row.url.ends_with(suffix)).unwrap();assert_eq!(get("added").change,"Added");assert!(get("added").baseline.is_none());assert!(get("removed").current.is_none());assert_eq!(get("same").change,"Unchanged");assert_eq!(get("changed").fields,vec!["headings","status","title","word_count"]);assert_eq!(get("changed").baseline.as_ref().unwrap()["title"],"ישן");assert_eq!(get("changed").current.as_ref().unwrap()["title"],"חדש");
 }
 #[test]fn incoming_edges_are_compared_but_link_order_and_timing_are_not(){
  let a=Link{url:"https://e.test/target".into(),kind:"anchor".into(),text:"a".into(),..Default::default()};let b=Link{text:"b".into(),..a.clone()};let old=vec![Page{links:vec![a.clone(),b.clone()],elapsed_ms:10,..page("source","source")},page("target","target")];let reordered=vec![Page{links:vec![b,a],elapsed_ms:999,headers:BTreeMap::from([("date".into(),"different".into())]),..page("source","source")},page("target","target")];assert!(analyze(&reordered,&old).iter().all(|row|row.change=="Unchanged"));
  let mut new=reordered.clone();new[0].links.clear();let rows=analyze(&new,&old);assert_eq!(rows.iter().find(|row|row.url.ends_with("target")).unwrap().fields,vec!["unique_inlinks"]);
 }
 #[test]fn project_comparison_is_read_only_and_checks_saved_seeds(){
  let dir=std::env::temp_dir().join(format!("fitig-compare-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir).unwrap();let current=dir.join("current.sqlite");let baseline=dir.join("baseline.sqlite");
  for(path,title)in[(&current,"חדש"),(&baseline,"ישן")]{let c=crate::open(path.to_str().unwrap()).unwrap();c.execute("INSERT INTO metadata VALUES('seed','https://e.test/')",[]).unwrap();let p=page("page",title);c.execute("INSERT INTO pages VALUES(?1,?2)",rusqlite::params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}
  let before=std::fs::read(&baseline).unwrap();let report=projects(current.to_str().unwrap(),baseline.to_str().unwrap()).unwrap();assert_eq!(report.counts["Changed"],1);assert_eq!(report.current_seed,"https://e.test/");assert!(std::fs::read(&baseline).unwrap()==before);std::fs::remove_dir_all(dir).unwrap();
 }
}

#[cfg(test)]mod response_hash_comparison_tests{
 use super::*;
 #[test]fn compare_only_two_observed_payloads(){
  let old=Page{url:"https://example.test/".into(),..Default::default()};
  let mut new=old.clone();new.response_hash=Some("abc".into());
  assert_eq!(analyze(&[new.clone()],&[old.clone()])[0].change,"Unchanged");
  let mut known=old;known.response_hash=Some("def".into());
  assert_eq!(analyze(&[new],&[known])[0].fields,vec!["response_hash"]);
 }
}

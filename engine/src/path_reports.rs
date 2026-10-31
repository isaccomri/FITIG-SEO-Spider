//! Independent reports derived from saved redirect/canonical observations only.
use crate::{Page,Result};
use serde::Serialize;
use std::collections::{BTreeMap,BTreeSet};
pub const KINDS:[&str;6]=["all-redirects","redirect-chains","redirect-canonical-chains","redirects-to-error","canonical-chains","non-indexable-canonicals"];
#[derive(Debug,Serialize)]pub struct Report{pub kind:String,pub headers:Vec<String>,pub rows:Vec<Vec<String>>,pub truncated:bool,pub scope:String}
struct PathResult{path:Vec<String>,edges:Vec<&'static str>,outcome:&'static str,status:Option<u16>}
fn follow<'a>(start:&'a Page,by:&BTreeMap<&str,&'a Page>,canonical:bool,mixed:bool)->PathResult{
 let mut path=vec![start.url.clone()];let mut edges=Vec::new();let mut seen=BTreeSet::from([start.url.as_str()]);let mut current=start;
 for _ in 0..1000{
  let (next,edge)=if !canonical&&!current.redirect.is_empty(){(current.redirect.as_str(),"redirect")}else if (canonical||mixed)&&!current.canonical.is_empty()&&current.canonical!=current.url{(current.canonical.as_str(),"canonical")}else{return PathResult{path,edges,outcome:if current.status==0{"No response"}else if current.status>=400{"HTTP error"}else if (300..400).contains(&current.status){"Redirect without recorded target"}else if crate::directives::restriction(current,"googlebot","noindex"){"Noindex"}else{"Terminal observation"},status:Some(current.status)}};
  edges.push(edge);path.push(next.to_owned());if !seen.insert(next){return PathResult{path,edges,outcome:"Loop",status:by.get(next).map(|page|page.status)}}
  match by.get(next){None=>return PathResult{path,edges,outcome:"Target not crawled",status:None},Some(target)=>{current=target;if canonical&&(300..400).contains(&current.status){return PathResult{path,edges,outcome:"Canonical target redirects",status:Some(current.status)}}}}
 }
 PathResult{path,edges,outcome:"1000-hop safety limit",status:Some(current.status)}
}
pub fn build(pages:&[Page],kind:&str)->Result<Report>{
 if !KINDS.contains(&kind){return Err("Unknown redirect/canonical report".into())}
 let by:BTreeMap<_,_>=pages.iter().map(|page|(page.url.as_str(),page)).collect();let canonical=matches!(kind,"canonical-chains"|"non-indexable-canonicals");let mixed=kind=="redirect-canonical-chains";
 let mut report=Report{kind:kind.into(),headers:["Source","Source Status","Final URL","Final Status","Hops","Outcome","Path (JSON)","Relationships (JSON)"].into_iter().map(String::from).collect(),rows:Vec::new(),truncated:false,scope:"Saved observations only. Uncrawled targets have blank final status; they are not HTTP errors. Chains use recorded targets; 1000-hop and 50000-row safety limits are explicit. Non-indexable means observed error, no response, noindex, redirect or loop; it is not an indexing decision. No network requests.".into()};
 for page in pages{if canonical{if page.canonical.is_empty()||page.canonical==page.url{continue}}else if page.redirect.is_empty(){continue}
  let result=follow(page,&by,canonical,mixed);let include=match kind{"redirect-chains"|"canonical-chains"=>result.edges.len()>1||matches!(result.outcome,"Loop"|"1000-hop safety limit"),"redirects-to-error"=>result.status.is_some_and(|status|status>=400),"non-indexable-canonicals"=>matches!(result.outcome,"HTTP error"|"No response"|"Noindex"|"Canonical target redirects"|"Loop"),_=>true};if !include{continue}
  if report.rows.len()==50000{report.truncated=true;break}
  report.rows.push(vec![page.url.clone(),page.status.to_string(),result.path.last().cloned().unwrap_or_default(),result.status.map(|status|status.to_string()).unwrap_or_default(),result.edges.len().to_string(),result.outcome.into(),serde_json::to_string(&result.path)?,serde_json::to_string(&result.edges)?]);
 }Ok(report)
}
pub fn load(db:&str,kind:&str)->Result<Report>{build(&crate::load_project_data(db)?.pages,kind)}
#[cfg(test)]mod tests{
 use super::*;
 fn page(path:&str,status:u16)->Page{Page{url:format!("https://e.test/{path}"),status,..Default::default()}}
 #[test]fn redirects_distinguish_single_chain_errors_and_unknowns(){let mut a=page("a",301);a.redirect="https://e.test/b".into();let mut b=page("b",302);b.redirect="https://e.test/שגיאה".into();let c=page("שגיאה",404);let mut d=page("d",301);d.redirect="https://e.test/unknown".into();let pages=vec![a,b,c,d];let all=build(&pages,"all-redirects").unwrap();assert_eq!(all.rows.len(),3);assert_eq!(all.rows[2][3],"");assert_eq!(all.rows[2][5],"Target not crawled");let chains=build(&pages,"redirect-chains").unwrap();assert_eq!(chains.rows.len(),1);assert_eq!(chains.rows[0][4],"2");assert_eq!(build(&pages,"redirects-to-error").unwrap().rows.len(),2);assert!(all.rows[0][6].contains("שגיאה"));}
 #[test]fn mixed_paths_follow_canonical_and_loops_are_bounded(){let mut a=page("a",301);a.redirect="https://e.test/b".into();let mut b=page("b",200);b.canonical="https://e.test/c".into();let mut c=page("c",200);c.canonical="https://e.test/b".into();let pages=vec![a,b,c];let mixed=build(&pages,"redirect-canonical-chains").unwrap();assert_eq!(mixed.rows[0][5],"Loop");assert_eq!(mixed.rows[0][4],"3");assert_eq!(build(&pages,"canonical-chains").unwrap().rows.len(),2);assert_eq!(build(&pages,"non-indexable-canonicals").unwrap().rows.len(),2);assert_eq!(build(&pages,"all-redirects").unwrap().rows[0][4],"1");}
 #[test]fn non_indexable_uses_observations_not_missing_targets(){let mut a=page("a",200);a.canonical="https://e.test/b".into();let b=Page{directives:"noindex".into(),..page("b",200)};let mut c=page("c",200);c.canonical="https://e.test/unknown".into();assert_eq!(build(&[a,b,c],"non-indexable-canonicals").unwrap().rows.len(),1);assert!(build(&[],"invalid").is_err());}
}

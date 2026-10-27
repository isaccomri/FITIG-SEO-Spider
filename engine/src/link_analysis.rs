//! Independent internal-link ranking and evidence-qualified orphan candidates.
use crate::{Page,Result};
use serde::Serialize;
use std::collections::{BTreeMap,BTreeSet,VecDeque};
use url::Url;
#[derive(Debug,Serialize)]
pub struct Row{
 pub url:String,pub status:Option<u16>,pub sources:Vec<String>,pub reachable:bool,
 pub orphan_candidate:bool,pub evidence:String,pub unique_inlinks:usize,
 pub followed_inlinks:usize,pub followed_outlinks:usize,pub rank:Option<f64>,pub score:Option<f64>,
}
#[derive(Debug,Serialize)]
pub struct Report{pub rows:Vec<Row>,pub scored_pages:usize,pub iterations:usize,pub residual:f64,pub converged:bool,pub note:String}
fn normalized(raw:&str)->Option<String>{let mut url=Url::parse(raw).ok()?;if !matches!(url.scheme(),"http"|"https")||!url.username().is_empty()||url.password().is_some(){return None}url.set_fragment(None);Some(url.to_string())}
fn nofollow(value:&str)->bool{value.split(|character:char|character==','||character.is_whitespace()).any(|token|token.eq_ignore_ascii_case("nofollow")||token.eq_ignore_ascii_case("none"))}
pub fn analyze(pages:&[Page],seed:&str,inventory:&[(String,String)])->Result<Report>{
 analyze_scoped(pages,seed,inventory,"")
}
pub fn analyze_scoped(pages:&[Page],seed:&str,inventory:&[(String,String)],root:&str)->Result<Report>{
 let seed=normalized(seed).ok_or("Link analysis needs an absolute HTTP(S) seed")?;let scope=crate::scope::Scope::new(&Url::parse(&seed)?,root)?;
 let internal=|raw:&str|Url::parse(raw).map(|url|scope.contains(&url)).unwrap_or(false);
 let observed:BTreeMap<_,_>=pages.iter().filter_map(|page|normalized(&page.url).map(|url|(url,page))).collect();
 let nodes:Vec<_>=observed.iter().filter(|(url,page)|internal(url)&&(200..300).contains(&page.status)&&page.content_type.contains("html")).map(|(url,_)|url.clone()).collect();
 if nodes.len()>100000{return Err("Link scoring is limited to 100,000 successful HTML pages".into())}
 let positions:BTreeMap<_,_>=nodes.iter().enumerate().map(|(index,url)|(url.as_str(),index)).collect();
 let mut edges=vec![BTreeSet::<usize>::new();nodes.len()];let mut reachable_edges:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();let mut incoming:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();let mut followed_incoming=vec![BTreeSet::new();nodes.len()];let mut edge_count=0;
 for(url,page)in &observed{
  if !internal(url){continue}
  let adjacency=reachable_edges.entry(url.clone()).or_default();
  if let Some(target)=normalized(&page.redirect).filter(|target|internal(target)){adjacency.insert(target);}
  for link in page.links.iter().filter(|link|link.kind=="anchor"){
   let Some(target)=normalized(&link.url).filter(|target|internal(target))else{continue};
   adjacency.insert(target.clone());incoming.entry(target.clone()).or_default().insert(url.clone());
   if nofollow(&page.directives)||nofollow(&link.rel){continue}
   if let(Some(&source),Some(&destination))=(positions.get(url.as_str()),positions.get(target.as_str())){
    if edges[source].insert(destination){edge_count+=1;followed_incoming[destination].insert(source);if edge_count>1000000{return Err("Link scoring is limited to 1,000,000 unique followed edges".into())}}
   }
  }
 }
 let mut reached=BTreeSet::new();let mut queue=VecDeque::from([seed]);while let Some(url)=queue.pop_front(){if !reached.insert(url.clone()){continue}if let Some(targets)=reachable_edges.get(&url){for target in targets{if !reached.contains(target){queue.push_back(target.clone())}}}}
 let n=nodes.len();let mut rank=if n>0{vec![1.0/n as f64;n]}else{vec![]};let mut iterations=0;let mut residual=0.0;
 if n>0{for iteration in 1..=100{
  let dangling: f64=rank.iter().zip(&edges).filter(|(_,targets)|targets.is_empty()).map(|(value,_)|*value).sum();
  let mut next=vec![(0.15+0.85*dangling)/n as f64;n];for(source,targets)in edges.iter().enumerate(){if !targets.is_empty(){let share=0.85*rank[source]/targets.len() as f64;for &target in targets{next[target]+=share}}}
  residual=next.iter().zip(&rank).map(|(a,b)|(a-b).abs()).sum();rank=next;iterations=iteration;if residual<1e-10{break}
 }}
 let maximum=rank.iter().copied().fold(0.0,f64::max);let mut known:BTreeMap<String,BTreeSet<String>>=nodes.iter().map(|url|(url.clone(),BTreeSet::from(["Crawl".into()]))).collect();
 for(raw,source)in inventory{if let Some(url)=normalized(raw).filter(|url|internal(url)){known.entry(url).or_default().insert(source.clone());}}
 let rows=known.into_iter().map(|(url,sources)|{
  let position=positions.get(url.as_str()).copied();let page=observed.get(&url);let reachable=reached.contains(&url);
  Row{status:page.and_then(|page|if page.status>0{Some(page.status)}else{None}),sources:sources.into_iter().collect(),reachable,orphan_candidate:!reachable,
   evidence:if reachable{"Path observed from seed"}else if page.is_some(){"No seed path in stored crawl graph"}else{"Inventory URL not crawled; no observed seed path"}.into(),
   unique_inlinks:incoming.get(&url).map(BTreeSet::len).unwrap_or(0),followed_inlinks:position.map(|index|followed_incoming[index].len()).unwrap_or(0),followed_outlinks:position.map(|index|edges[index].len()).unwrap_or(0),rank:position.map(|index|rank[index]),score:position.map(|index|100.0*rank[index]/maximum),url}
 }).collect();
 Ok(Report{rows,scored_pages:n,iterations,residual,converged:residual<1e-10,note:"Scores use an independent PageRank-style model (damping 0.85; internal successful HTML; unique followed edges; dangling redistribution; linear maximum 100). They are not certified as Screaming Frog Link Score. Reachability includes nofollow hyperlinks and HTTP redirects. Orphan candidates mean no seed path in this saved graph; partial crawls, List mode and missing sources cannot prove site-wide orphan status. Uncrawled inventory URLs have no invented status or score.".into()})
}
pub fn project(db:&str)->Result<Report>{
 let pages=crate::load_pages(db)?;let seed=crate::project_seed(db)?;let mut inventory=Vec::new();
 if let Some(sitemaps)=crate::project_sitemaps(db)?{for url in sitemaps["locations"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_str){inventory.push((url.into(),"Sitemap".into()))}}
 for(service,label)in[("ga4","GA4"),("gsc","Search Console")]{if let Some(report)=crate::integrations::load_report(db,service)?{for row in report.rows{inventory.push((row.url,label.into()))}}}
 analyze_scoped(&pages,&seed,&inventory,&crate::project_profile(db)?.subdomain_root)
}
#[cfg(test)]mod tests{
 use super::*;use crate::Link;
 #[test]fn explicit_root_scores_cross_subdomain_links_and_inventory(){let seed="https://www.example.co.il/";let child="https://shop.example.co.il/page";let pages=vec![Page{url:seed.into(),status:200,content_type:"text/html".into(),links:vec![Link{url:child.into(),kind:"anchor".into(),internal:true,..Default::default()}],..Default::default()},Page{url:child.into(),status:200,content_type:"text/html".into(),..Default::default()}];let report=analyze_scoped(&pages,seed,&[("https://cdn.example.co.il/unseen".into(),"Sitemap".into()),("https://badexample.co.il/".into(),"Sitemap".into())],"example.co.il").unwrap();assert_eq!(report.scored_pages,2);assert_eq!(report.rows.len(),3);let row=report.rows.iter().find(|row|row.url==child).unwrap();assert!(row.reachable);assert_eq!(row.unique_inlinks,1);assert_eq!(analyze(&pages,seed,&[]).unwrap().scored_pages,1);}
 fn page(path:&str,targets:&[(&str,&str)])->Page{Page{url:format!("https://e.test/{path}"),status:200,content_type:"text/html".into(),links:targets.iter().map(|(target,rel)|Link{url:format!("https://e.test/{target}"),rel:rel.to_string(),kind:"anchor".into(),internal:true,..Default::default()}).collect(),..Default::default()}}
 #[test]fn stable_mass_and_nofollow_reachability(){
  let pages=[page("",&[("a",""),("a",""),("b","nofollow")]),page("a",&[("","" )]),page("b",&[])];let report=analyze(&pages,"https://e.test/",&[]).unwrap();assert!(report.converged);assert!((report.rows.iter().filter_map(|row|row.rank).sum::<f64>()-1.0).abs()<1e-9);assert!(report.rows.iter().all(|row|row.reachable&&!row.orphan_candidate));let b=report.rows.iter().find(|row|row.url.ends_with("/b")).unwrap();assert_eq!(b.unique_inlinks,1);assert_eq!(b.followed_inlinks,0);let root=report.rows.iter().find(|row|row.url=="https://e.test/").unwrap();assert_eq!(root.followed_outlinks,1);assert!(report.rows.iter().filter_map(|row|row.score).all(|score|score>0.0&&score<=100.0));
 }
 #[test]fn disconnected_clusters_and_inventory_are_qualified_candidates(){
  let pages=[page("",&[]),page("a",&[("b","")]),page("b",&[("a","")])];let inventory=vec![("https://e.test/missing".into(),"Sitemap".into()),("https://e.test/missing#x".into(),"GA4".into()),("https://outside.test/".into(),"Sitemap".into())];let report=analyze(&pages,"https://e.test/",&inventory).unwrap();assert_eq!(report.rows.len(),4);assert_eq!(report.rows.iter().filter(|row|row.orphan_candidate).count(),3);let missing=report.rows.iter().find(|row|row.url.ends_with("missing")).unwrap();assert_eq!(missing.sources,vec!["GA4","Sitemap"]);assert!(missing.status.is_none()&&missing.rank.is_none());assert_eq!(report.rows.iter().find(|row|row.url.ends_with("/a")).unwrap().unique_inlinks,1);
 }
 #[test]fn redirect_seed_and_empty_graph(){let mut redirect=page("",&[]);redirect.status=301;redirect.redirect="https://e.test/home".into();let report=analyze(&[redirect,page("home",&[])],"https://e.test/",&[]).unwrap();assert!(report.rows[0].reachable);let empty=analyze(&[],"https://e.test/",&[]).unwrap();assert!(empty.rows.is_empty()&&empty.converged);assert_eq!(empty.iterations,0);}
 #[test]fn combines_saved_sitemap_and_api_inventory_without_inventing_observations(){
  let path=std::env::temp_dir().join(format!("fitig-link-analysis-{}-{}.sqlite",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let db=path.to_string_lossy().into_owned();
  {let connection=crate::open(&db).unwrap();connection.execute("INSERT INTO metadata VALUES('seed','https://e.test/')",[]).unwrap();let root=page("",&[]);connection.execute("INSERT INTO pages VALUES(?1,?2)",rusqlite::params![root.url,serde_json::to_string(&root).unwrap()]).unwrap();}
  crate::save_sitemaps(&db,serde_json::json!({"locations":["https://e.test/שלום"],"files":["sitemap.xml"],"missing":[]})).unwrap();
  crate::integrations::save_report(&db,&crate::integrations::ApiReport{service:"ga4".into(),property:"fixture".into(),start:"2026-01-01".into(),end:"2026-01-02".into(),strategy:String::new(),rows:vec![crate::integrations::ApiRow{url:"https://e.test/שלום".into(),metrics:BTreeMap::new()}],truncated:false,note:"Offline fixture".into()}).unwrap();
  let report=project(&db).unwrap();let candidate=report.rows.iter().find(|row|row.orphan_candidate).unwrap();assert_eq!(candidate.sources,vec!["GA4","Sitemap"]);assert!(candidate.status.is_none()&&candidate.score.is_none());assert_eq!(report.scored_pages,1);
  std::fs::remove_file(path).unwrap();
 }
}

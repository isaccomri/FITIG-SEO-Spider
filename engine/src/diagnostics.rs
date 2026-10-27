//! Post-crawl checks use only stored observations. Unfetched targets remain unknown.
use crate::Page;
use serde_json::{json,Value};
use std::collections::{BTreeMap,BTreeSet};
pub fn analyze(pages:&[Page])->Value{
 let by_url:BTreeMap<_,_>=pages.iter().map(|p|(p.url.as_str(),p)).collect();let mut canonicals=Vec::new();let mut redirects=Vec::new();let mut hreflang=Vec::new();let mut title_groups:BTreeMap<String,Vec<String>>=BTreeMap::new();let mut descriptions:BTreeMap<String,Vec<String>>=BTreeMap::new();let mut issues:BTreeMap<String,usize>=BTreeMap::new();let mut incoming=BTreeSet::new();
 for page in pages{
  if page.internal&&page.content_type.contains("html")&&(200..300).contains(&page.status){if !page.title.is_empty(){title_groups.entry(page.title.clone()).or_default().push(page.url.clone());}if !page.description.is_empty(){descriptions.entry(page.description.clone()).or_default().push(page.url.clone());}}
  for issue in &page.issues{*issues.entry(issue.clone()).or_default()+=1;}for link in page.links.iter().filter(|l|l.kind=="anchor"){incoming.insert(link.url.clone());}
  if !page.canonical.is_empty()&&page.canonical!=page.url{
   let mut current=page.canonical.as_str();let mut seen=BTreeSet::from([page.url.as_str()]);let mut chain=vec![page.url.clone()];let mut issue="Canonical target not crawled";
   loop{chain.push(current.into());if !seen.insert(current){issue="Canonical loop";break}let Some(target)=by_url.get(current)else{break};if target.status>=400{issue="Canonical target HTTP error";break}if (300..400).contains(&target.status){issue="Canonical target redirects";break}if crate::directives::restriction(target,"googlebot","noindex"){issue="Canonical target noindex";break}if target.canonical.is_empty()||target.canonical==target.url{issue=if chain.len()>2{"Canonical chain"}else{""};break}current=&target.canonical;if chain.len()>100{issue="Canonical chain exceeds 100 hops";break}}
   if !issue.is_empty(){canonicals.push(json!({"url":page.url,"issue":issue,"chain":chain}));}
  }
  if !page.redirect.is_empty(){let mut chain=vec![page.url.clone()];let mut seen=BTreeSet::from([page.url.as_str()]);let mut current=page.redirect.as_str();let mut issue="Redirect target not crawled";loop{chain.push(current.into());if !seen.insert(current){issue="Redirect loop";break}let Some(target)=by_url.get(current)else{break};if target.status>=400{issue="Redirect ends at HTTP error";break}if target.redirect.is_empty(){issue=if chain.len()>2{"Redirect chain"}else{"Single redirect"};break}current=&target.redirect;if chain.len()>100{issue="Redirect chain exceeds 100 hops";break}}redirects.push(json!({"url":page.url,"issue":issue,"chain":chain}));}
  let mut languages=BTreeMap::new();for(lang,target_url)in &page.hreflang{
   let lang_key=lang.to_ascii_lowercase();if let Some(previous)=languages.insert(lang_key.clone(),target_url){if previous!=target_url{hreflang.push(json!({"url":page.url,"target":target_url,"issue":"Conflicting targets for hreflang","language":lang}));}}
   let valid=lang_key=="x-default"||regex::Regex::new(r"^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$").unwrap().is_match(&lang_key);if !valid{hreflang.push(json!({"url":page.url,"target":target_url,"issue":"Invalid hreflang syntax","language":lang}));}
   if target_url==&page.url{continue}let issue=match by_url.get(target_url.as_str()){None=>Some("Hreflang target not crawled"),Some(target)if target.status>=400=>Some("Hreflang target HTTP error"),Some(target)if (300..400).contains(&target.status)=>Some("Hreflang target redirects"),Some(target)if !target.hreflang.iter().any(|(_,url)|url==&page.url)=>Some("Missing hreflang return link"),_=>None};if let Some(issue)=issue{hreflang.push(json!({"url":page.url,"target":target_url,"issue":issue,"language":lang}));}
  }
 }
 title_groups.retain(|_,urls|urls.len()>1);descriptions.retain(|_,urls|urls.len()>1);
 let candidates=pages.iter().filter(|page|page.internal&&page.depth>0&&page.content_type.contains("html")&&!incoming.contains(&page.url)).map(|page|page.url.clone()).collect::<Vec<_>>();
 json!({"canonical_issues":canonicals,"redirect_paths":redirects,"hreflang_issues":hreflang,"duplicate_titles":title_groups,"duplicate_descriptions":descriptions,"issue_counts":issues,"no_observed_hyperlink_inlinks":candidates,"scope_note":"Checks use stored observations only. Uncrawled targets are unknown; no-inlink candidates are not proven site-wide orphans. Hreflang syntax is structural, not a language registry validation."})
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn detects_loops_error_targets_and_duplicate_metadata(){let a=Page{url:"https://e.test/a".into(),status:200,title:"Same".into(),content_type:"text/html".into(),canonical:"https://e.test/b".into(),redirect:"https://e.test/b".into(),..Default::default()};let b=Page{url:"https://e.test/b".into(),canonical:a.url.clone(),redirect:a.url.clone(),title:"Same".into(),status:200,content_type:"text/html".into(),..Default::default()};let result=analyze(&[a,b]);assert_eq!(result["canonical_issues"][0]["issue"],"Canonical loop");assert_eq!(result["redirect_paths"][0]["issue"],"Redirect loop");assert_eq!(result["duplicate_titles"]["Same"].as_array().unwrap().len(),2)}
 #[test]fn unknown_hreflang_is_not_broken_and_missing_return_is_reported(){let a=Page{url:"https://e.test/a".into(),hreflang:vec![("he".into(),"https://e.test/b".into()),("en".into(),"https://e.test/unknown".into())],..Default::default()};let b=Page{url:"https://e.test/b".into(),status:200,..Default::default()};let result=analyze(&[a,b]);assert_eq!(result["hreflang_issues"][0]["issue"],"Missing hreflang return link");assert_eq!(result["hreflang_issues"][1]["issue"],"Hreflang target not crawled")}
}

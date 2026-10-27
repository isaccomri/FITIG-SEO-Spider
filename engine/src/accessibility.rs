//! Versioned, bounded axe-core observations. Missing/error reports are not passing audits.
use crate::Result;
use serde::{Serialize,Deserialize};
use rusqlite::OptionalExtension;
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct Counts{pub violations:usize,pub incomplete:usize,pub passes:usize,pub inapplicable:usize,pub violation_nodes:usize,pub incomplete_nodes:usize}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Row{pub rule:String,pub status:String,pub impact:String,pub help:String,pub help_url:String,pub tags:Vec<String>,pub selector:String,pub html:String,pub summary:String}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Report{pub version:String,pub url:String,pub counts:Option<Counts>,pub rows:Vec<Row>,pub error:String,pub truncated:bool}
pub fn failed(error:&str)->Report{Report{version:"4.14.0".into(),url:String::new(),counts:None,rows:vec![],error:error.chars().take(2000).collect(),truncated:false}}
pub fn script()->String{
 let library=include_str!("../../assets/third-party/axe-core/axe.min.js");
 format!("(async()=>{{\n{library}\n{}\n}})()",include_str!("accessibility-run.js"))
}
pub fn validate(report:&Report)->Result<()>{
 if report.version!="4.14.0"||report.rows.len()>80||serde_json::to_vec(report)?.len()>65536{return Err("Accessibility report exceeds supported version/size limits".into())}
 if report.error.is_empty(){let url=url::Url::parse(&report.url)?;if !matches!(url.scheme(),"http"|"https")||report.counts.is_none(){return Err("Accessibility success requires an observed HTTP URL and counts".into())}}else if report.counts.is_some()||!report.rows.is_empty(){return Err("A failed accessibility audit must not contain passing counts or findings".into())}
 for row in &report.rows{if !["Violation","Needs review"].contains(&row.status.as_str())||row.rule.len()>256||row.tags.len()>16||row.html.len()>2048||row.selector.len()>4096||row.summary.len()>8192{return Err("Unsupported accessibility finding".into())}}
 Ok(())
}
pub fn save(connection:&rusqlite::Connection,url:&str,report:&Report)->Result<()>{validate(report)?;connection.execute("INSERT OR REPLACE INTO metadata(key,value) VALUES(?1,?2)",rusqlite::params![format!("accessibility:{url}"),serde_json::to_string(report)?])?;Ok(())}
pub fn load(db:&str,url:&str)->Result<Option<Report>>{let c=crate::open_read(db)?;let raw:Option<String>=c.query_row("SELECT value FROM metadata WHERE key=?1",[format!("accessibility:{url}")],|r|r.get(0)).optional()?;raw.map(|raw|{if raw.len()>65536{return Err("Saved accessibility report exceeds 64 KiB".into())}let report=serde_json::from_str(&raw)?;validate(&report)?;Ok(report)}).transpose()}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn failure_and_unknown_are_not_success(){let mut report=failed("deadline exceeded");validate(&report).unwrap();assert!(report.counts.is_none());report.counts=Some(Default::default());assert!(validate(&report).is_err());report.error.clear();report.url="file:///test".into();assert!(validate(&report).is_err());report.url="https://e.test/".into();assert!(validate(&report).is_ok());report.version="different".into();assert!(validate(&report).is_err());}
 #[test]fn script_uses_pinned_offline_engine_and_explicit_result_bounds(){let source=script();assert!(source.contains("4.14.0"));assert!(source.contains("iframes:false"));assert!(source.contains("preload:false"));assert!(source.contains("28000"));assert!(source.contains("wcag22aa"));}
}

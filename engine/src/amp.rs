//! Bounded, read-only AMP validation inputs and official-validator output.
use serde::{Deserialize,Serialize};
pub const MAX_SOURCE:usize=2*1024*1024;
#[derive(Debug,Deserialize,Serialize)]pub struct Finding{pub severity:String,pub code:String,pub line:u32,pub col:u32,pub message:String,pub spec_url:String}
#[derive(Debug,Deserialize,Serialize)]pub struct Report{pub status:String,pub total:usize,pub truncated:bool,pub findings:Vec<Finding>}
pub fn source(db:&str,url:&str)->crate::Result<String>{let html=crate::load_source(db,url)?.ok_or("No recorded HTTP source for this URL")?;if html.len()>MAX_SOURCE{return Err("AMP source exceeds 2 MiB validation limit".into())}Ok(html)}
pub fn decode(value:serde_json::Value)->Result<Report,String>{
 let report:Report=serde_json::from_value(value).map_err(|e|e.to_string())?;
 if !["PASS","FAIL"].contains(&report.status.as_str())||report.findings.len()>80||report.total<report.findings.len()||report.truncated!=(report.total>report.findings.len())||report.status=="PASS"&&report.findings.iter().any(|f|f.severity=="ERROR")||report.findings.iter().any(|f|!matches!(f.severity.as_str(),"ERROR"|"WARNING")||f.code.len()>256||f.message.len()>4096||f.spec_url.len()>2048){return Err("Invalid AMP validator report".into())}Ok(report)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn rejects_unknown_and_inconsistent_results(){for value in [serde_json::json!({"status":"UNKNOWN","total":0,"truncated":false,"findings":[]}),serde_json::json!({"status":"FAIL","total":1,"truncated":false,"findings":[]})]{assert!(decode(value).is_err())}}
 #[test]fn preserves_failure_evidence(){let report=decode(serde_json::json!({"status":"FAIL","total":1,"truncated":false,"findings":[{"severity":"ERROR","code":"TEST","line":3,"col":2,"message":"עברית","spec_url":"https://amp.dev/"}]})).unwrap();assert_eq!(report.findings[0].line,3);assert_eq!(report.findings[0].message,"עברית")}
}

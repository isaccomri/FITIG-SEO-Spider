//! Bounded observations from Chromium Runtime console and exception events.
use crate::Result;
use serde::{Serialize,Deserialize};
use serde_json::Value;
use rusqlite::OptionalExtension;
#[derive(Debug,Clone,Serialize,Deserialize)]pub struct Entry{#[serde(default)]pub truncated:bool,pub level:String,pub message:String,pub source:String,pub line:Option<u32>,pub timestamp_ms:Option<f64>}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]pub struct Report{pub entries:Vec<Entry>,pub dropped:usize,pub error:String}
fn bounded(text:&str,limit:usize)->String{text.chars().take(limit).collect()}
fn location(value:&Value)->(String,Option<u32>){let frame=&value["stackTrace"]["callFrames"][0];let source=frame["url"].as_str().or_else(||value["url"].as_str()).unwrap_or("");let line=frame["lineNumber"].as_u64().or_else(||value["lineNumber"].as_u64()).and_then(|line|u32::try_from(line).ok()).and_then(|line|line.checked_add(1));(bounded(source,8192),line)}
impl Report{
 pub fn record_raw(&mut self,event:&str,text:&str){if self.entries.len()>=200||text.len()>256*1024{self.dropped+=1;return}match serde_json::from_str(text){Ok(value)=>self.record(event,&value),Err(_)=>{self.dropped+=1;self.error="A browser diagnostic event could not be decoded".into()}}}
 pub fn record(&mut self,event:&str,value:&Value){
  if self.entries.len()>=200{self.dropped+=1;return}
  let mut truncated=false;
  let (level,message,source,line)=if event=="Runtime.consoleAPICalled"{
   let level=match value["type"].as_str().unwrap_or("log"){"warning"=>"Warning","error"=>"Error","assert"=>"Assert","info"=>"Info","debug"=>"Debug","log"=>"Log",other=>other};
   let arguments=value["args"].as_array();truncated=arguments.is_some_and(|args|args.len()>50);
   let parts:Vec<String>=arguments.into_iter().flatten().take(50).map(|argument|{let text=if let Some(text)=argument["value"].as_str(){text.to_string()}else if !argument["value"].is_null(){argument["value"].to_string()}else{argument["description"].as_str().or_else(||argument["unserializableValue"].as_str()).unwrap_or("null").to_string()};if text.chars().count()>4000{truncated=true}bounded(&text,4000)}).collect();let text=parts.join(" ");if text.chars().count()>4000{truncated=true}let(source,line)=location(value);(bounded(level,64),bounded(&text,4000),source,line)
  }else if event=="Runtime.exceptionThrown"{
   let detail=&value["exceptionDetails"];let message=detail["exception"]["description"].as_str().or_else(||detail["text"].as_str()).unwrap_or("JavaScript exception");if message.chars().count()>4000{truncated=true}let(source,line)=location(detail);("Exception".into(),bounded(message,4000),source,line)
  }else{return};
  self.entries.push(Entry{truncated,level,message,source,line,timestamp_ms:value["timestamp"].as_f64().filter(|time|time.is_finite())});
 }
}
pub fn load(db:&str,url:&str)->Result<Option<Report>>{let c=crate::open_read(db)?;let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='browser_logs')",[],|r|r.get(0))?;if !exists{return Ok(None)}let raw:Option<String>=c.query_row("SELECT json FROM browser_logs WHERE url=?1",[url],|r|r.get(0)).optional()?;raw.map(|raw|serde_json::from_str(&raw).map_err(Into::into)).transpose()}
#[cfg(test)]mod tests{
 use super::*;use serde_json::json;
 #[test]fn parses_unicode_console_values_and_one_based_source_lines(){let mut report=Report::default();report.record("Runtime.consoleAPICalled",&json!({"type":"warning","args":[{"value":"שלום"},{"value":42},{"description":"Object"}],"timestamp":1234.5,"stackTrace":{"callFrames":[{"url":"https://e.test/app.js","lineNumber":3}]}}));assert_eq!(report.entries[0].level,"Warning");assert_eq!(report.entries[0].message,"שלום 42 Object");assert_eq!(report.entries[0].line,Some(4));assert_eq!(report.entries[0].timestamp_ms,Some(1234.5));}
 #[test]fn exception_and_absent_source_remain_observed_or_unknown(){let mut report=Report::default();report.record("Runtime.exceptionThrown",&json!({"exceptionDetails":{"exception":{"description":"Error: בדיקה"},"url":"https://e.test/","lineNumber":5}}));assert_eq!(report.entries[0].level,"Exception");assert_eq!(report.entries[0].line,Some(6));assert!(report.entries[0].message.contains("בדיקה"));report.record("Runtime.consoleAPICalled",&json!({"args":[{"value":false}]}));assert_eq!(report.entries[1].line,None);assert_eq!(report.entries[1].source,"");assert_eq!(report.entries[1].message,"false");}
 #[test]fn event_and_message_limits_report_dropped_observations(){let mut report=Report::default();let message="שלום".repeat(2000);report.record("Runtime.consoleAPICalled",&json!({"args":[{"value":message}]}));assert_eq!(report.entries[0].message.chars().count(),4000);assert!(report.entries[0].truncated);for _ in 0..201{report.record("Runtime.consoleAPICalled",&json!({"args":[]}))}assert_eq!(report.entries.len(),200);assert_eq!(report.dropped,2);let mut report=Report::default();report.record_raw("Runtime.consoleAPICalled","not json");assert_eq!(report.dropped,1);assert!(!report.error.is_empty());}
}

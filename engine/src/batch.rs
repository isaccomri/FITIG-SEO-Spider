//! Explicit sequential HTTP jobs. Every run owns a newly created output directory.
use crate::{Result,Url,CrawlOptions,CrawlProfile};
use serde::{Serialize,Deserialize};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}}};
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {pub version:u32,pub jobs:Vec<Job>,#[serde(default)]pub exports:Vec<String>,#[serde(default)]pub continue_on_error:bool}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {#[serde(default="timeout")]pub request_timeout_secs:u64,pub name:String,pub url:String,#[serde(default)]pub urls:Option<Vec<String>>,#[serde(default)]pub limit:usize,#[serde(default="delay")]pub delay_ms:u64,#[serde(default="concurrency")]pub concurrency:usize}
fn timeout()->u64{20}fn delay()->u64{200}fn concurrency()->usize{5}
#[derive(Clone,Serialize,Deserialize)]
pub struct JobReport {pub name:String,pub status:String,pub database:String,pub completed:usize,pub queued:usize,pub error:String,pub exports:Vec<String>}
#[derive(Clone,Serialize,Deserialize)]
pub struct Report {pub output:String,pub status:String,pub jobs:Vec<JobReport>}
fn target(raw:&str)->Result<String>{let url=Url::parse(raw.trim()).map_err(|_|"Batch URLs must be absolute HTTP(S)")?;if !matches!(url.scheme(),"http"|"https")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.fragment().is_some(){return Err("Batch URLs must be absolute HTTP(S), without credentials or fragments".into())}Ok(url.to_string())}
pub fn validate(mut manifest:Manifest)->Result<Manifest>{
 if serde_json::to_vec(&manifest)?.len()>1024*1024{return Err("Batch manifest exceeds 1 MiB".into())}
 if manifest.version!=1||manifest.jobs.is_empty()||manifest.jobs.len()>20{return Err("Batch version 1 supports 1–20 explicit jobs".into())}
 if manifest.exports.len()>4||manifest.exports.iter().any(|format|!["csv","html","xlsx","sitemap"].contains(&format.as_str())){return Err("Batch exports must be csv, html, xlsx or sitemap".into())}
 let mut formats=std::collections::HashSet::new();if manifest.exports.iter().any(|value|!formats.insert(value)){return Err("Batch export formats must not repeat".into())}
 for job in &mut manifest.jobs{crate::validate_request_timeout(job.request_timeout_secs)?;if job.name.trim().is_empty()||job.name.len()>128||job.name.chars().any(char::is_control){return Err("Use a job name of 1–128 bytes without control characters".into())}job.url=target(&job.url)?;if !(1..=32).contains(&job.concurrency)||job.delay_ms>60000{return Err("Job concurrency must be 1–32 and delay at most 60,000 ms".into())}if let Some(urls)=&mut job.urls{if urls.is_empty()||urls.len()>10000{return Err("A batch List job supports 1–10,000 explicit URLs".into())}for url in urls.iter_mut(){*url=target(url)?}if urls[0]!=job.url{return Err("A List job URL must equal its first explicit URL".into())}}}
 Ok(manifest)
}
fn save(root:&Path,report:&Report)->Result<()>{std::fs::write(root.join("batch-status.json"),serde_json::to_vec_pretty(report)?)?;Ok(())}
pub fn output_path(output:&str)->Result<PathBuf>{let requested=PathBuf::from(output);let absolute=if requested.is_absolute(){requested}else{std::env::current_dir()?.join(requested)};let name=absolute.file_name().filter(|value|*value!="."&&*value!="..").ok_or("Choose a new named batch directory")?;let parent=absolute.parent().ok_or("Batch output requires an existing parent directory")?.canonicalize()?;if !parent.is_dir(){return Err("Batch parent must be a directory".into())}let root=parent.join(name);if root.exists(){return Err("Batch output folder already exists; choose a new folder".into())}Ok(root)}
pub fn run(manifest:Manifest,output:&str,cancel:Arc<AtomicBool>,mut emit:impl FnMut(&Report))->Result<Report>{
 let manifest=validate(manifest)?;let root=output_path(output)?;
 // create_dir, not create_dir_all: no reuse, overwrite, deletion or guessed unique path.
 std::fs::create_dir(&root)?;let root_string=root.to_string_lossy().into_owned();
 std::fs::write(root.join("batch-manifest.json"),serde_json::to_vec_pretty(&manifest)?)?;
 let mut report=Report{output:root_string,status:"running".into(),jobs:manifest.jobs.iter().enumerate().map(|(index,job)|JobReport{name:job.name.clone(),status:"queued".into(),database:root.join(format!("job-{:03}",index+1)).join("crawl.sqlite").to_string_lossy().into_owned(),completed:0,queued:0,error:String::new(),exports:Vec::new()}).collect()};save(&root,&report)?;emit(&report);
 let mut halted=false;
 for (index,job) in manifest.jobs.iter().enumerate(){
  if cancel.load(Ordering::Relaxed)||root.join("batch.stop").exists(){cancel.store(true,Ordering::Relaxed);report.jobs[index].status="skipped".into();continue}
  if halted{report.jobs[index].status="skipped".into();continue}
  let db=report.jobs[index].database.clone();std::fs::create_dir(Path::new(&db).parent().unwrap())?;report.jobs[index].status="running".into();save(&root,&report)?;emit(&report);
  let result=(||->Result<()>{
   let profile=CrawlProfile{request_timeout_secs:job.request_timeout_secs,concurrency:job.concurrency,..Default::default()};
   crate::crawl_configured(CrawlOptions{url:job.url.clone(),db:db.clone(),limit:job.limit,delay_ms:job.delay_ms,resume:false},job.urls.clone(),profile,|progress|{
    report.jobs[index].completed=progress.completed;report.jobs[index].queued=progress.queued;
    if cancel.load(Ordering::Relaxed)||root.join("batch.stop").exists(){cancel.store(true,Ordering::Relaxed);let _=std::fs::write(format!("{db}.stop"),b"stop");}
    emit(&report);
   })?;
   report.jobs[index].completed=crate::load_pages(&db)?.len();report.jobs[index].queued=crate::pending_count(&db)?;
   if cancel.load(Ordering::Relaxed)||root.join("batch.stop").exists(){cancel.store(true,Ordering::Relaxed);report.jobs[index].status="stopped".into();return Ok(())}
   for format in &manifest.exports{let extension=if format=="sitemap"{"xml"}else{format};let out=Path::new(&db).parent().unwrap().join(format!("report.{extension}"));crate::export(&db,out.to_str().ok_or("Output path must be Unicode")?,format)?;report.jobs[index].exports.push(out.to_string_lossy().into_owned());}
   report.jobs[index].status=if report.jobs[index].queued>0{"limited"}else{"completed"}.into();Ok(())
  })();
  if let Err(error)=result{if let Ok(pages)=crate::load_pages(&db){report.jobs[index].completed=pages.len();}if let Ok(queued)=crate::pending_count(&db){report.jobs[index].queued=queued;}report.jobs[index].status="failed".into();report.jobs[index].error=error.to_string();if !manifest.continue_on_error{halted=true;}}
  save(&root,&report)?;emit(&report);
 }
 report.status=if cancel.load(Ordering::Relaxed){"stopped"}else if report.jobs.iter().any(|job|job.status=="failed"){"failed"}else if report.jobs.iter().any(|job|job.status=="limited"){"limited"}else{"completed"}.into();save(&root,&report)?;emit(&report);Ok(report)
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn validates_explicit_jobs_and_rejects_hidden_credentials_or_extra_fields(){let valid=serde_json::json!({"version":1,"jobs":[{"name":"עברית","url":"https://e.test/עברית"}],"exports":["csv"]});let m:Manifest=serde_json::from_value(valid.clone()).unwrap();assert!(validate(m).unwrap().jobs[0].url.contains("%D7"));let mut secret=valid.clone();secret["jobs"][0]["credential"]=serde_json::json!("not-allowed");assert!(serde_json::from_value::<Manifest>(secret).is_err());for url in ["file:///test","https://u:p@e.test/","https://e.test/#a"]{let mut value=valid.clone();value["jobs"][0]["url"]=serde_json::json!(url);assert!(validate(serde_json::from_value(value).unwrap()).is_err())}}
 #[test]fn validates_limits_lists_formats_before_creating_output(){let value=serde_json::json!({"version":1,"jobs":[{"name":"a","url":"https://e.test/","concurrency":33}]});assert!(validate(serde_json::from_value(value).unwrap()).is_err());let value=serde_json::json!({"version":1,"jobs":[{"name":"a","url":"https://e.test/","urls":["https://e.test/other"]}],"exports":["csv","csv"]});assert!(validate(serde_json::from_value(value).unwrap()).is_err());}
}

#[cfg(test)]mod timeout_tests{use super::*;#[test]fn batch_timeout_defaults_and_bounds(){for(seconds,valid)in[(0,false),(1,true),(300,true),(301,false)]{let input=serde_json::json!({"version":1,"jobs":[{"name":"timeout","url":"https://example.test/","request_timeout_secs":seconds}]});assert_eq!(validate(serde_json::from_value(input).unwrap()).is_ok(),valid)}let input=serde_json::json!({"version":1,"jobs":[{"name":"legacy","url":"https://example.test/"}]});assert_eq!(validate(serde_json::from_value(input).unwrap()).unwrap().jobs[0].request_timeout_secs,20);}}

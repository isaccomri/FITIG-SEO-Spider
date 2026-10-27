//! Chromium DOM rendering in an isolated, per-page browser profile.
use crate::Result;
use serde::{Serialize,Deserialize};
use std::{path::PathBuf,process::{Command,Stdio},time::{Duration,Instant}};
/// DOM and browser location are separate from the initial HTTP observation.
#[derive(Debug,Clone,Default)]
pub struct RenderedDocument {pub mobile:Option<crate::mobile::Report>,pub accessibility:Option<crate::accessibility::Report>,pub html:String,pub url:String,pub screenshot_png:Option<Vec<u8>>,pub screenshot_error:String,pub console:Option<crate::browser_logs::Report>,pub javascript:Option<crate::javascript::Report>}
impl From<String> for RenderedDocument {
 fn from(html:String)->Self{Self{html,..Default::default()}}
}
impl RenderedDocument {
 pub fn validated(mut self,requested:&str)->Result<Self>{
  let mut location=url::Url::parse(if self.url.is_empty(){requested}else{&self.url})?;
  if !matches!(location.scheme(),"http"|"https")||location.host_str().is_none(){return Err("Renderer location must be an absolute HTTP(S) URL".into())}
  location.set_fragment(None);self.url=location.to_string();
  if self.html.len()>10*1024*1024{return Err("Rendered DOM exceeds 10 MiB limit".into())}
  if self.html.trim().is_empty(){return Err("Renderer returned an empty DOM".into())}if let Some(bytes)=&self.screenshot_png{if let Err(error)=crate::screenshots::dimensions(bytes){self.screenshot_error=error.to_string();self.screenshot_png=None;}}Ok(self)
 }
}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(default)]
pub struct Rendering {pub capture_mobile:bool,pub capture_accessibility:bool,pub custom_javascript:Vec<crate::javascript::Rule>,pub capture_console:bool,pub capture_screenshot:bool,pub viewport_width:u32,pub viewport_height:u32,pub enabled:bool,pub binary:String,pub wait_ms:u64,pub timeout_ms:u64}
impl Default for Rendering{fn default()->Self{Self{capture_mobile:false,capture_accessibility:false,custom_javascript:Vec::new(),capture_console:false,capture_screenshot:false,viewport_width:1365,viewport_height:768,enabled:false,binary:String::new(),wait_ms:1500,timeout_ms:30000}}}
pub fn detect_chromium()->Option<String>{[
 "C:/Program Files/Google/Chrome/Application/chrome.exe",
 "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
 "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
 "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
 ].into_iter().find(|p|std::path::Path::new(p).is_file()).map(String::from)}
pub fn validate(options:&Rendering)->Result<()>{crate::javascript::validate(&options.custom_javascript)?;if options.capture_mobile&&(!options.enabled||!(320..=600).contains(&options.viewport_width)){return Err("Mobile checks require enabled built-in rendering and a 320–600 CSS-pixel viewport".into())}if options.capture_accessibility&&!options.enabled{return Err("Accessibility auditing requires built-in Windows WebView2 rendering to be enabled".into())}if !options.enabled{return Ok(())}if !(320..=4096).contains(&options.viewport_width)||!(240..=4096).contains(&options.viewport_height){return Err("Rendering viewport must be 320–4096 by 240–4096 pixels".into())}if (options.capture_mobile||options.capture_accessibility||options.capture_screenshot||options.capture_console||!options.custom_javascript.is_empty())&&!options.binary.is_empty(){return Err("Mobile checks, accessibility, screenshots, console capture and custom scripts require built-in Windows WebView2 rendering".into())}if options.wait_ms>30000||!(5000..=120000).contains(&options.timeout_ms){return Err("Rendering wait must be 0–30000 ms; timeout must be 5000–120000 ms".into())}if !options.binary.is_empty(){let name=std::path::Path::new(&options.binary).file_name().and_then(|p|p.to_str()).unwrap_or("").to_ascii_lowercase();if !["chrome.exe","msedge.exe"].contains(&name.as_str())||!std::path::Path::new(&options.binary).is_file(){return Err("External rendering requires an existing chrome.exe or msedge.exe".into())}}Ok(())}
struct Scratch(PathBuf);
impl Drop for Scratch{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
pub fn render(url:&str,options:&Rendering)->Result<String>{render_with_user_agent(url,options,"")}
pub fn render_with_user_agent(url:&str,options:&Rendering,user_agent:&str)->Result<String>{
 validate(options)?;if options.capture_mobile||options.capture_accessibility||options.capture_screenshot||options.capture_console||!options.custom_javascript.is_empty(){return Err("Mobile checks, accessibility, screenshots, console capture and custom scripts require built-in Windows WebView2 rendering".into())}let parsed=url::Url::parse(url)?;if !["http","https"].contains(&parsed.scheme()){return Err("Rendering supports HTTP(S) URLs only".into())}
 let binary=if options.binary.is_empty(){detect_chromium().ok_or("No Chromium browser found")?}else{options.binary.clone()};
 let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();let folder=std::env::temp_dir().join(format!("fitig-chromium-{}-{nonce}",std::process::id()));std::fs::create_dir(&folder)?;let scratch=Scratch(folder);let output=scratch.0.join("dom.html");let error_log=scratch.0.join("chromium.log");
 let mut command=Command::new(binary);command.args(["--headless=new","--disable-gpu","--disable-extensions","--no-first-run","--no-default-browser-check","--disable-background-networking","--dump-dom"])
 .arg(format!("--window-size={},{}",options.viewport_width,options.viewport_height)).arg(format!("--user-data-dir={}",scratch.0.join("profile").display())).arg(format!("--virtual-time-budget={}",options.wait_ms)).arg(format!("--timeout={}",options.timeout_ms)).arg(url)
 .stdin(Stdio::null()).stdout(Stdio::from(std::fs::File::create(&output)?)).stderr(Stdio::from(std::fs::File::create(&error_log)?));
 #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
 if !user_agent.is_empty(){command.arg(format!("--user-agent={user_agent}"));}
 let mut child=command.spawn()?;let start=Instant::now();let status=loop{if let Some(status)=child.try_wait()?{break status}if start.elapsed()>Duration::from_millis(options.timeout_ms){
  // The PID belongs to this isolated browser instance, never the user's open browser.
  #[cfg(windows)]{let mut kill=Command::new("taskkill.exe");use std::os::windows::process::CommandExt;let _=kill.args(["/PID",&child.id().to_string(),"/T","/F"]).creation_flags(0x08000000).stdout(Stdio::null()).stderr(Stdio::null()).status();}
  let _=child.kill();let _=child.wait();return Err("Chromium rendering timed out; owned browser process stopped".into())
 }std::thread::sleep(Duration::from_millis(50));};
 if !status.success(){let error=std::fs::read_to_string(error_log).unwrap_or_default();let diagnostic=error.chars().take(2000).collect::<String>();return Err(format!("Chromium failed with exit status {status}: {diagnostic}").into())}if std::fs::metadata(&output)?.len()>10*1024*1024{return Err("Rendered DOM exceeds 10 MiB limit".into())}
 let dom=std::fs::read_to_string(output)?;if dom.trim().is_empty(){return Err("Chromium returned an empty DOM".into())}Ok(dom)
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn renderer_location_validation_and_legacy_callback(){
  let old=RenderedDocument::from("<html>שלום</html>".to_string()).validated("https://example.test/start").unwrap();
  assert_eq!(old.url,"https://example.test/start");
  let current=RenderedDocument{html:"<html>ok</html>".into(),url:"https://example.test/final#section".into(),..Default::default()}.validated("https://example.test/start").unwrap();
  assert_eq!(current.url,"https://example.test/final");
  for location in ["chrome-error://chromewebdata/","file:///C:/secret","javascript:alert(1)","/relative"]{
   assert!(RenderedDocument{html:"<html>ok</html>".into(),url:location.into(),..Default::default()}.validated("https://example.test/").is_err());
  }
  assert!(RenderedDocument::from(String::new()).validated("https://example.test/").is_err());
 }
 #[test]fn rejects_invalid_renderer_settings(){let mut r=Rendering{enabled:true,binary:"C:/Windows/System32/cmd.exe".into(),..Default::default()};assert!(validate(&r).is_err());r.enabled=false;assert!(validate(&r).is_ok())}}

#[cfg(test)]mod screenshot_option_tests{
 use super::*;
 #[test]fn legacy_profiles_default_to_no_capture_and_viewports_are_validated(){let old:Rendering=serde_json::from_str(r#"{"enabled":true,"wait_ms":500,"timeout_ms":10000}"#).unwrap();assert!(!old.capture_screenshot);assert_eq!((old.viewport_width,old.viewport_height),(1365,768));assert!(validate(&old).is_ok());for(width,height)in[(0,768),(1365,0),(4097,768),(1365,4097)]{assert!(validate(&Rendering{viewport_width:width,viewport_height:height,..old.clone()}).is_err());}}
 #[test]fn screenshot_artifact_failure_does_not_discard_valid_rendered_dom(){let document=RenderedDocument{html:"<html>שלום</html>".into(),url:"https://e.test/".into(),screenshot_png:Some(b"not png".to_vec()),..Default::default()}.validated("https://e.test/").unwrap();assert!(document.html.contains("שלום"));assert!(document.screenshot_png.is_none());assert!(!document.screenshot_error.is_empty());}
}

//! Ephemeral Basic credentials: deliberately not serializable into a crawl profile.
use crate::Result;
use base64::Engine;
use serde::Deserialize;
use url::Url;
#[derive(Clone,Deserialize)]
pub struct Session{origin:String,username:String,password:String}
impl std::fmt::Debug for Session{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("HttpBasicSession { credentials: <redacted> }")}}
pub fn normalize_origin(raw:&str)->Result<String>{let url=Url::parse(raw.trim()).map_err(|_|"Supply an absolute HTTP(S) authentication origin")?;if !matches!(url.scheme(),"http"|"https")||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.path()!="/"||url.query().is_some()||url.fragment().is_some(){return Err("Authentication origin must be HTTP(S) scheme, hostname and optional port only".into())}Ok(url.origin().ascii_serialization())}
impl Session{
 pub fn new(origin:&str,username:&str,password:&str)->Result<Self>{let session=Self{origin:normalize_origin(origin)?,username:username.into(),password:password.into()};session.validate()?;Ok(session)}
 fn validate(&self)->Result<()>{normalize_origin(&self.origin)?;if self.username.is_empty()||self.username.len()>512||self.username.contains(':')||self.username.chars().any(char::is_control)||self.password.len()>4096||self.password.chars().any(char::is_control){return Err("Basic authentication needs a nonempty username up to 512 bytes without colon/control characters and a password up to 4096 bytes without control characters".into())}Ok(())}
 pub(crate) fn header_for(&self,url:&str)->Option<reqwest::header::HeaderValue>{if Url::parse(url).ok()?.origin().ascii_serialization()!=normalize_origin(&self.origin).ok()?{return None}let encoded=base64::engine::general_purpose::STANDARD.encode(format!("{}:{}",self.username,self.password));let mut header=reqwest::header::HeaderValue::from_str(&format!("Basic {encoded}")).ok()?;header.set_sensitive(true);Some(header)}
}
pub fn validate_configuration(origin:&str,session:Option<&Session>,rendering:bool)->Result<()>{
 if origin.is_empty(){if session.is_some(){return Err("Disable or clear the authentication session when the profile has no authentication origin".into())}return Ok(())}
 let expected=normalize_origin(origin)?;let session=session.ok_or("Enter HTTP Basic credentials for this session before starting or resuming the saved project")?;session.validate()?;
 if normalize_origin(&session.origin)?!=expected{return Err("Authentication session origin differs from the configured project origin".into())}
 if rendering{return Err("HTTP Basic authentication currently requires HTTP-only crawling; disable JavaScript rendering".into())}Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn credentials_are_exact_origin_scoped_and_debug_is_redacted(){let session=Session::new("https://example.test:443/","משתמש","סיסמה:42").unwrap();for url in ["https://example.test/a","https://example.test:443/robots.txt"]{let value=session.header_for(url).unwrap();assert!(value.is_sensitive());let decoded=base64::engine::general_purpose::STANDARD.decode(value.to_str().unwrap().strip_prefix("Basic ").unwrap()).unwrap();assert_eq!(String::from_utf8(decoded).unwrap(),"משתמש:סיסמה:42");assert!(!format!("{value:?}").contains("Basic"))}for url in ["http://example.test/","https://child.example.test/","https://example.test:444/","https://badexample.test/","https://example.test.evil.test/"]{assert!(session.header_for(url).is_none())}let debug=format!("{session:?}");assert!(!debug.contains("סיסמה")&&!debug.contains("משתמש"));}
 #[test]fn invalid_origins_and_unsafe_credential_shapes_are_rejected(){for origin in ["example.test","https://example.test/path","https://user:pw@example.test/","https://example.test/?x","https://example.test/#x","file:///tmp"]{assert!(Session::new(origin,"user","password").is_err())}for user in ["","with:colon","bad\r\nuser"]{assert!(Session::new("https://example.test",user,"password").is_err())}assert!(Session::new("https://example.test","user","bad\npassword").is_err());assert!(Session::new("https://example.test","user","").is_ok());}
 #[test]fn profiles_require_a_matching_session_and_explicitly_reject_unsupported_rendering(){let session=Session::new("https://example.test","user","password").unwrap();assert!(validate_configuration("https://example.test/",Some(&session),false).is_ok());assert!(validate_configuration("https://example.test",None,false).is_err());assert!(validate_configuration("https://different.test",Some(&session),false).is_err());assert!(validate_configuration("https://example.test",Some(&session),true).is_err());assert!(validate_configuration("",Some(&session),false).is_err());assert!(validate_configuration("",None,true).is_ok());}
}

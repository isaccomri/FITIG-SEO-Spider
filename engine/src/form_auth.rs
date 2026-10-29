//! Explicit standard HTML form login. Credentials and login bodies are never persisted.
use crate::{CrawlProfile,Result,http_cookies::Jar};
use serde::{Deserialize,Serialize};
use std::{io::Read,sync::Arc,time::Duration};
use url::Url;
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct Config{pub login_url:String,pub verify_url:String,pub success_text:String,pub username_field:String,pub password_field:String,pub csrf_field:String}
#[derive(Clone,Deserialize)]pub struct Credentials{pub username:String,pub password:String}
impl std::fmt::Debug for Credentials{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("FormCredentials(<redacted>)")}}
fn endpoint(raw:&str)->Result<Url>{let u=Url::parse(raw).map_err(|_|"Invalid form authentication URL")?;let local=matches!(u.host_str(),Some("localhost"|"127.0.0.1"|"[::1]"));if !(u.scheme()=="https"||u.scheme()=="http"&&local)||u.host_str().is_none()||!u.username().is_empty()||u.password().is_some()||u.fragment().is_some(){return Err("Form authentication requires HTTPS (or literal localhost HTTP), without URL credentials or fragments".into())}Ok(u)}
pub fn validate(profile:&CrawlProfile,credentials:Option<&Credentials>,seed:&str)->Result<()>{
 let Some(c)=&profile.form_auth else{if credentials.is_some(){return Err("Form credentials supplied without configuration".into())}return Ok(())};
 let login=endpoint(&c.login_url)?;let verify=endpoint(&c.verify_url)?;let seed=Url::parse(seed).map_err(|_|"Invalid crawl seed")?;
 if login.origin()!=verify.origin()||login.origin()!=seed.origin(){return Err("Form login, verification and crawl seed must use the same exact origin".into())}
 if profile.rendering.enabled||!profile.cookies_enabled||!profile.http_auth_origin.is_empty()||profile.proxy_mode!="direct"{return Err("Form login requires HTTP-only crawling, enabled cookies, Direct proxy mode and disabled Basic authentication".into())}
 for field in [&c.username_field,&c.password_field,&c.csrf_field]{if field.len()>128||field.chars().any(|ch|ch.is_control()){return Err("Invalid form field name".into())}}
 if c.username_field.is_empty()||c.password_field.is_empty()||c.username_field==c.password_field||!c.csrf_field.is_empty()&&(c.csrf_field==c.username_field||c.csrf_field==c.password_field){return Err("Form field names must be distinct; username and password names are required".into())}
 if c.success_text.trim().is_empty()||c.success_text.len()>512||c.success_text.chars().any(|ch|ch.is_control()){return Err("Supply a nonempty verification text marker (up to 512 bytes)".into())}
 let credentials=credentials.ok_or("Re-enter form credentials for this crawl; credentials are not saved")?;
 if credentials.username.is_empty()||credentials.password.is_empty()||credentials.username.len()>4096||credentials.password.len()>4096||credentials.username.contains('\0')||credentials.password.contains('\0'){return Err("Supply bounded nonempty form credentials".into())}Ok(())
}
fn read(response:reqwest::blocking::Response)->Result<String>{let mut bytes=Vec::new();response.take(1024*1024+1).read_to_end(&mut bytes).map_err(|_|"Form authentication response could not be read; body omitted")?;if bytes.len()>1024*1024{return Err("Form authentication response exceeds 1 MiB".into())}String::from_utf8(bytes).map_err(|_|"Form authentication currently requires UTF-8 HTML".into())}
pub fn authenticate(profile:&CrawlProfile,credentials:&Credentials)->Result<Arc<Jar>>{
 let config=profile.form_auth.as_ref().ok_or("Form login configuration missing")?;validate(profile,Some(credentials),&config.login_url)?;
 let login=endpoint(&config.login_url)?;let jar=Arc::new(Jar::new_scoped(&login.origin().ascii_serialization())?);
 let client=reqwest::blocking::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(profile.request_timeout_secs)).user_agent(&profile.user_agent).cookie_provider(jar.clone()).build().map_err(|_|"Form HTTP client unavailable")?;
 let response=client.get(login.clone()).send().map_err(|_|"Form login page request failed; details omitted")?;if !response.status().is_success(){return Err("Form login GET must return a successful response without redirect".into())}let html=read(response)?;
 let mut fields=vec![(config.username_field.clone(),credentials.username.clone()),(config.password_field.clone(),credentials.password.clone())];
 if !config.csrf_field.is_empty(){let doc=scraper::Html::parse_document(&html);let selector=scraper::Selector::parse("input[name]").unwrap();let tokens:Vec<_>=doc.select(&selector).filter(|el|el.value().attr("name")==Some(config.csrf_field.as_str())).collect();if tokens.len()!=1{return Err("Expected exactly one configured CSRF input".into())}let token=tokens[0].value().attr("value").ok_or("CSRF input has no value")?;if token.len()>8192{return Err("CSRF value exceeds limit".into())}fields.push((config.csrf_field.clone(),token.into()));}
 let encoded=url::form_urlencoded::Serializer::new(String::new()).extend_pairs(fields.iter().map(|(k,v)|(k.as_str(),v.as_str()))).finish();
 let response=client.post(login).header(reqwest::header::CONTENT_TYPE,"application/x-www-form-urlencoded").body(encoded).send().map_err(|_|"Form submission failed; details omitted; submission was not retried")?;
 let status=response.status();if !status.is_success()&&!status.is_redirection(){return Err("Form submission was rejected; response body omitted".into())}drop(response);
 // Never follow the POST redirect: verification uses the explicit same-origin URL.
 let response=client.get(&config.verify_url).send().map_err(|_|"Form verification request failed; details omitted")?;if !response.status().is_success(){return Err("Form verification did not succeed".into())}
 let verified=read(response)?;if !verified.contains(&config.success_text){return Err("Form verification marker was not found; credentials were not accepted as authenticated".into())}
 if jar.report()?.cookies.is_empty(){return Err("Form login produced no usable cookies".into())}Ok(jar)
}
#[cfg(test)]mod tests{
 use super::*;
 fn profile()->CrawlProfile{CrawlProfile{form_auth:Some(Config{login_url:"https://e.test/login".into(),verify_url:"https://e.test/private".into(),success_text:"Signed in".into(),username_field:"user".into(),password_field:"pass".into(),csrf_field:"csrf".into()}),cookies_enabled:true,proxy_mode:"direct".into(),..Default::default()}}
 #[test]fn validates_origin_configuration_and_missing_credentials(){let p=profile();let credentials=Credentials{username:"user".into(),password:"secret".into()};assert!(validate(&p,Some(&credentials),"https://e.test/").is_ok());assert!(validate(&p,None,"https://e.test/").is_err());assert!(validate(&p,Some(&credentials),"https://other.test/").is_err());let mut bad=p.clone();bad.form_auth.as_mut().unwrap().verify_url="https://e.test:444/private".into();assert!(validate(&bad,Some(&credentials),"https://e.test/").is_err());bad=p.clone();bad.proxy_mode="automatic".into();assert!(validate(&bad,Some(&credentials),"https://e.test/").is_err());assert!(!format!("{credentials:?}").contains("secret"));}
}

//! Explicit hostname namespace; no guessed registrable-domain/public-suffix boundary.
use crate::Result;
use url::Url;

#[derive(Clone,Debug)]
pub struct Scope { origin: url::Origin, root: Option<String> }
impl Scope {
 pub fn new(seed:&Url, root:&str)->Result<Self> {
  let root=root.trim();
  if root.is_empty(){return Ok(Self{origin:seed.origin(),root:None})}
  if root.len()>253 || root.chars().any(|c|c.is_whitespace()||c.is_control()||"/:@?#%\\[]".contains(c)) {
   return Err("Subdomain root must be a hostname without scheme, port, credentials, path or escapes".into())
  }
  let parsed=Url::parse(&format!("https://{root}/"))?;
  let Some(url::Host::Domain(host))=parsed.host() else{return Err("Subdomain root must be a DNS hostname, not an IP address".into())};
  let host=host.trim_end_matches('.').to_ascii_lowercase();
  if host.is_empty() || host.split('.').any(|label|label.is_empty()||label.len()>63||label.starts_with('-')||label.ends_with('-')||!label.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')) || (!host.contains('.')&&host!="localhost") {
   return Err("Supply a DNS domain such as example.co.il (or localhost for local fixtures)".into())
  }
  let Some(url::Host::Domain(seed_host))=seed.host() else{return Err("Subdomain scope requires a DNS seed hostname".into())};
  if !within(seed_host,&host){return Err("The crawl seed hostname must be inside the explicit subdomain root".into())}
  Ok(Self{origin:seed.origin(),root:Some(host)})
 }
 pub fn contains(&self,url:&Url)->bool {
  if url.origin()==self.origin{return true}
  let Some(root)=&self.root else{return false};
  match (&self.origin,url.host()) {
   (url::Origin::Tuple(scheme,_,port),Some(url::Host::Domain(host)))=>url.scheme()==scheme&&url.port_or_known_default()==Some(*port)&&within(host,root),
   _=>false
  }
 }
}
fn within(host:&str,root:&str)->bool{let host=host.trim_end_matches('.');host.eq_ignore_ascii_case(root)||host.len()>root.len()&&host[host.len()-root.len()..].eq_ignore_ascii_case(root)&&host.as_bytes()[host.len()-root.len()-1]==b'.'}

#[cfg(test)]mod tests {
 use super::*;
 fn url(value:&str)->Url{Url::parse(value).unwrap()}
 #[test]fn default_scope_keeps_exact_origin_and_effective_ports(){let s=Scope::new(&url("https://www.example.co.il/a"),"").unwrap();for u in ["https://www.example.co.il/", "https://www.example.co.il:443/x"]{assert!(s.contains(&url(u)))}for u in ["https://example.co.il/","https://child.www.example.co.il/","http://www.example.co.il/","https://www.example.co.il:444/"]{assert!(!s.contains(&url(u)))}}
 #[test]fn explicit_root_includes_siblings_and_descendants_but_not_suffix_lookalikes(){let s=Scope::new(&url("https://www.example.co.il/"),"EXAMPLE.co.il.").unwrap();for u in ["https://example.co.il/","https://shop.example.co.il/","https://a.shop.example.co.il/","https://shop.example.co.il./"]{assert!(s.contains(&url(u)),"{u}")}for u in ["https://badexample.co.il/","https://example.co.il.evil.test/","https://co.il/","http://shop.example.co.il/","https://shop.example.co.il:444/","https://example.other/"]{assert!(!s.contains(&url(u)),"{u}")}}
 #[test]fn unicode_root_uses_idna_hostname_boundary(){let seed=url("https://www.דוגמה.ישראל/");let s=Scope::new(&seed,"דוגמה.ישראל").unwrap();assert!(s.contains(&url("https://חנות.דוגמה.ישראל/שלום")));assert!(s.contains(&url("https://דוגמה.ישראל/")));assert!(!s.contains(&url("https://לאדוגמה.ישראל/")))}
 #[test]fn invalid_or_unrelated_roots_are_rejected(){for root in ["https://example.test","example.test/path","example.test:443","user@example.test","example.test?x","example.test#x","example..test","-bad.test","bad-.test","example%2etest","example.test\\evil","[::1]","127.0.0.1","com","other.test"]{assert!(Scope::new(&url("https://www.example.test/"),root).is_err(),"{root}")}assert!(Scope::new(&url("http://127.0.0.1/"),"localhost").is_err());assert!(Scope::new(&url("http://localhost/"),"localhost").is_ok())}
}

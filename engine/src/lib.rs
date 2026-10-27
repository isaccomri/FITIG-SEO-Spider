pub mod google_sheets;
pub mod semantic;
use rusqlite::{params, Connection, OptionalExtension};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, time::Instant};
use url::Url;
#[cfg(test)]use std::time::Duration;
pub mod directives;
pub mod link_metrics;
pub mod spelling;
pub mod credential_vault;
pub mod amp;
pub mod diagnostics;
pub mod encoding;
pub mod sitemaps;
pub mod url_inspection;
pub mod chart_export;
pub mod batch;
pub mod accessibility;
pub mod mobile;
pub mod export_target;
pub mod sitemap_export;
pub mod structured_data;
pub mod pagination;
mod google_rules;
pub mod text_metrics;
pub mod link_analysis;
pub mod text_analysis;
pub mod crawl_compare;
pub mod screenshots;
pub mod browser_logs;
pub mod javascript;
mod database;
mod http_crawl;
pub mod scope;
pub mod rewriting;
pub mod authentication;
pub mod http_cookies;
pub mod proxy_settings;
mod protected_state;
pub mod integrations;
pub mod render;
pub mod custom;
pub use custom::{CustomRule,CustomResult,CustomProgram};
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Human-readable Unicode URL. Only non-ASCII UTF-8 escapes are decoded;
/// reserved URL separators, invalid bytes and bidi controls remain escaped.
pub fn display_url(value: &str) -> String {
 static ENCODED: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
 ENCODED.get_or_init(||regex::Regex::new(r"(?i)(?:%[89a-f][0-9a-f])+").unwrap())
 .replace_all(value, |capture: &regex::Captures<'_>| {
  match percent_encoding::percent_decode_str(&capture[0]).decode_utf8() {
   Ok(decoded) if !decoded.chars().any(|c|c.is_control()||matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')) => decoded.into_owned(),
   _ => capture[0].to_string(),
  }
 }).into_owned()
}

fn transient_io(kind: std::io::ErrorKind) -> bool {
 matches!(kind, std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
  | std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::TimedOut)
}
fn transient_request(error: &reqwest::Error) -> bool {
 if error.is_timeout() { return true; }
 let mut source: Option<&(dyn Error + 'static)> = Some(error);
 while let Some(cause) = source {
  if let Some(io) = cause.downcast_ref::<std::io::Error>() { return transient_io(io.kind()); }
  source = cause.source();
 }
 false
}
#[cfg(test)]
fn retry_get<T,E>(mut send: impl FnMut() -> std::result::Result<T,E>, retryable: impl Fn(&E)->bool, mut wait: impl FnMut(Duration)) -> std::result::Result<T,E> {
 for attempt in 0..3 {
  match send() {
   Ok(value) => return Ok(value),
   Err(error) if attempt < 2 && retryable(&error) => wait(Duration::from_millis(100 * (1 << attempt))),
   Err(error) => return Err(error),
  }
 }
 unreachable!("all attempts return or retry within the bounded loop")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Link { pub url: String, pub text: String, pub rel: String, pub kind: String, pub internal: bool, pub alt: String, pub target: String, pub position: String, pub path_type: String }
fn default_internal()->bool{true}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
 #[serde(default)]pub html_extracted:Option<bool>,
 #[serde(default)]pub directive_rules:Option<Vec<crate::directives::Rule>>,
 #[serde(default="default_internal")] pub internal: bool,
 #[serde(default)] pub subdomain_root: String,
 pub url: String, pub status: u16, pub content_type: String, pub title: String, pub description: String,
 pub headings: Vec<Vec<String>>, pub canonical: String, pub directives: String, pub hreflang: Vec<(String,String)>,
 pub links: Vec<Link>, pub images: Vec<(String,String)>, pub json_ld: Vec<String>, pub word_count: usize,
 #[serde(default,skip_serializing_if="Option::is_none")] pub body_terms: Option<BTreeMap<String,usize>>,
 #[serde(default)] pub response_hash: Option<String>,
 pub content_hash: String, pub size: usize, pub elapsed_ms: u128, pub depth: usize, pub redirect: String,
 #[serde(default)] pub custom: Vec<CustomResult>,
 #[serde(default)] pub source_stored: bool,
 #[serde(default)]pub screenshot_stored:bool,
 #[serde(default)]pub screenshot_error:String,
 #[serde(default)]pub rewrite_count:Option<usize>,#[serde(default)] pub rewrite_errors:Option<usize>,#[serde(default)] pub javascript_count:Option<usize>,
 #[serde(default)]pub javascript_errors:Option<usize>,
 #[serde(default)]pub mobile_findings:Option<usize>,#[serde(default)]pub mobile_error:String,#[serde(default)]pub mobile_truncated:bool,
 #[serde(default)]pub accessibility_violations:Option<usize>,#[serde(default)]pub accessibility_nodes:Option<usize>,#[serde(default)]pub accessibility_review:Option<usize>,#[serde(default)]pub accessibility_error:String,#[serde(default)]pub accessibility_truncated:bool,
 #[serde(default)]pub console_count:Option<usize>,
 #[serde(default)]pub console_errors:Option<usize>,
 #[serde(default)]pub console_error:String,
 #[serde(default)] pub rendered: bool,
 #[serde(default)] pub render_error: String,
 #[serde(default)] pub rendered_url: String,
 #[serde(default)] pub charset: String,
 #[serde(default)] pub charset_source: String,
 #[serde(default)] pub decoding_errors: bool,
 pub headers: BTreeMap<String,String>, pub issues: Vec<String>, pub error: String,
}
impl Default for Page{fn default()->Self{Self{response_hash:None,html_extracted:None,directive_rules:None,mobile_findings:None,mobile_error:String::new(),mobile_truncated:false,accessibility_violations:None,accessibility_nodes:None,accessibility_review:None,accessibility_error:String::new(),accessibility_truncated:false,internal:true,subdomain_root:String::new(),url:String::new(),status:0,content_type:String::new(),title:String::new(),description:String::new(),headings:Vec::new(),canonical:String::new(),directives:String::new(),hreflang:Vec::new(),links:Vec::new(),images:Vec::new(),json_ld:Vec::new(),word_count:0,body_terms:None,content_hash:String::new(),size:0,elapsed_ms:0,depth:0,redirect:String::new(),custom:Vec::new(),source_stored:false,screenshot_stored:false,screenshot_error:String::new(),rewrite_count:None,rewrite_errors:None,javascript_count:None,javascript_errors:None,console_count:None,console_errors:None,console_error:String::new(),rendered:false,render_error:String::new(),rendered_url:String::new(),charset:String::new(),charset_source:String::new(),decoding_errors:false,headers:BTreeMap::new(),issues:Vec::new(),error:String::new()}}}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlOptions { pub url: String, pub db: String, pub limit: usize, pub delay_ms: u64, pub resume: bool }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(default)]
pub struct CrawlProfile {pub request_timeout_secs:u64,pub concurrency:usize,pub custom_rules:Vec<CustomRule>,pub store_html:bool,pub check_external:bool,pub subdomain_root:String,pub http_auth_origin:String,pub cookies_enabled:bool,pub proxy_mode:String,pub proxy_url:String,pub proxy_bypass:String,pub proxy_auth_required:bool,pub url_rewriting:rewriting::Options,pub max_depth:usize,pub include_patterns:Vec<String>,pub exclude_patterns:Vec<String>,pub user_agent:String,pub rendering:render::Rendering}
impl Default for CrawlProfile{fn default()->Self{Self{request_timeout_secs:20,concurrency:5,custom_rules:Vec::new(),store_html:true,check_external:false,subdomain_root:String::new(),http_auth_origin:String::new(),cookies_enabled:false,proxy_mode:"automatic".into(),proxy_url:String::new(),proxy_bypass:String::new(),proxy_auth_required:false,url_rewriting:Default::default(),max_depth:0,include_patterns:Vec::new(),exclude_patterns:Vec::new(),user_agent:"FITIGBot/0.1.49".into(),rendering:Default::default()}}}
pub fn validate_request_timeout(seconds:u64)->Result<()>{if !(1..=300).contains(&seconds){return Err("HTTP request timeout must be between 1 and 300 seconds".into())}Ok(())}
pub fn project_profile(db:&str)->Result<CrawlProfile>{let value:Option<String>=open_read(db)?.query_row("SELECT value FROM metadata WHERE key='profile'",[],|r|r.get(0)).optional()?;Ok(match value{Some(v)=>serde_json::from_str(&v)?,None=>CrawlProfile::default()})}
pub fn project_sitemaps(db:&str)->Result<Option<serde_json::Value>>{let value:Option<String>=open_read(db)?.query_row("SELECT value FROM metadata WHERE key='sitemaps'",[],|r|r.get(0)).optional()?;value.map(|v|serde_json::from_str(&v).map_err(Into::into)).transpose()}
pub fn save_sitemaps(db:&str,inventory:serde_json::Value)->Result<()>{let locations=inventory["locations"].as_array().ok_or("Sitemap inventory needs locations")?;if locations.len()>250000{return Err("Sitemap inventory exceeds 250,000 locations".into())}for value in locations{let url=Url::parse(value.as_str().ok_or("Invalid sitemap URL")?)?;if !["http","https"].contains(&url.scheme())||url.fragment().is_some()||url.password().is_some()||!url.username().is_empty(){return Err("Sitemap locations must be absolute HTTP(S) URLs without fragments or credentials".into())}}let serialized=serde_json::to_string(&inventory)?;if serialized.len()>64*1024*1024{return Err("Sitemap inventory exceeds 64 MiB".into())}open(db)?.execute("INSERT OR REPLACE INTO metadata VALUES('sitemaps',?1)",[serialized])?;Ok(())}
pub fn load_source(db:&str,url:&str)->Result<Option<String>>{let connection=open_read(db)?;let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='documents')",[],|r|r.get(0))?;if !exists{return Ok(None)}Ok(connection.query_row("SELECT html FROM documents WHERE url=?1",[url],|r|r.get(0)).optional()?)}
pub fn load_rendered_source(db:&str,url:&str)->Result<Option<String>>{let connection=open_read(db)?;let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='rendered_documents')",[],|r|r.get(0))?;if !exists{return Ok(None)}Ok(connection.query_row("SELECT html FROM rendered_documents WHERE url=?1",[url],|r|r.get(0)).optional()?)}
pub fn reprocess_custom(db:&str,rules:Vec<CustomRule>)->Result<usize>{
 let program=CustomProgram::compile(&rules)?;let pages=load_pages(db)?;
 let sources=pages.iter().map(|page|Ok((page.clone(),if page.rendered{load_rendered_source(db,&page.url)?}else{load_source(db,&page.url)?}))).collect::<Result<Vec<_>>>()?;
 if !sources.iter().any(|(_,source)|source.is_some()){return Err("This crawl has no stored HTML. Run a new crawl with Store HTML enabled.".into())}
 let mut profile=project_profile(db)?;profile.custom_rules=rules;let mut connection=open(db)?;let tx=connection.transaction()?;let mut changed=0;
 for(mut page,source)in sources{if let Some(html)=source{page.custom=program.run(&html);tx.execute("UPDATE pages SET json=?2 WHERE url=?1",params![page.url,serde_json::to_string(&page)?])?;changed+=1}}
 tx.execute("INSERT OR REPLACE INTO metadata VALUES('profile',?1)",[serde_json::to_string(&profile)?])?;tx.commit()?;Ok(changed)
}
#[derive(Debug, Serialize)]
pub struct Progress { pub event: String, pub completed: usize, pub queued: usize, pub page: Option<Page> }
fn sel(s: &str) -> Selector { Selector::parse(s).unwrap() }
fn absolute(base: &Url, raw: &str) -> Option<String> { let mut u=base.join(raw).ok()?; if !["http","https"].contains(&u.scheme()) {return None}; u.set_fragment(None); Some(u.to_string()) }
fn text(el: scraper::ElementRef<'_>) -> String { el.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ") }
fn extracted_text(el:scraper::ElementRef<'_>,content_area:bool)->String{el.descendants().filter_map(|n|{let value=n.value().as_text()?;if n.ancestors().filter_map(|a|a.value().as_element()).any(|e|["script","style","noscript","template"].contains(&e.name())||(content_area&&["nav","footer"].contains(&e.name()))){None}else{Some(value.to_string())}}).collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")}
fn visible_text(el:scraper::ElementRef<'_>)->String{extracted_text(el,false)}
fn content_text(el:scraper::ElementRef<'_>)->String{extracted_text(el,true)}
fn body_terms(content:&str)->BTreeMap<String,usize>{let mut counts=BTreeMap::new();for term in content.split(|c:char|!c.is_alphanumeric()).map(str::to_lowercase).filter(|s|s.chars().count()>=2){*counts.entry(term).or_insert(0usize)+=1;}let mut sorted=counts.into_iter().collect::<Vec<_>>();sorted.sort_by(|a,b|b.1.cmp(&a.1).then_with(||a.0.cmp(&b.0)));sorted.truncate(512);sorted.into_iter().collect()}
fn link_metadata(el:scraper::ElementRef<'_>,raw:&str)->Link {
 let position=el.ancestors().filter_map(|n|n.value().as_element()).find_map(|e|match e.name(){"nav"=>Some("Navigation"),"header"=>Some("Header"),"footer"=>Some("Footer"),"aside"=>Some("Sidebar"),"head"=>Some("Head"),"main"=>Some("Content"),_=>match e.attr("role"){Some("navigation")=>Some("Navigation"),Some("banner")=>Some("Header"),Some("contentinfo")=>Some("Footer"),Some("complementary")=>Some("Sidebar"),_=>None}}).unwrap_or("Content");
 let path_type=if raw.starts_with("//"){"Protocol-relative"}else if Url::parse(raw).is_ok(){"Absolute"}else if raw.starts_with('/') {"Root-relative"}else{"Path-relative"};
 Link{alt:el.select(&sel("img[alt]")).filter_map(|e|e.value().attr("alt")).collect::<Vec<_>>().join(" | "),target:el.value().attr("target").unwrap_or("").into(),position:position.into(),path_type:path_type.into(),..Default::default()}
}
pub fn extract(url: &str, html: &str, depth: usize) -> Result<Page> {
 let document_url=Url::parse(url)?; let doc=Html::parse_document(html);
 // Only the first href-bearing base participates. Invalid/data/javascript
 // bases fall back to the document URL, never to a later base element.
 let base=doc.select(&sel("base[href]")).next()
  .and_then(|element|document_url.join(element.value().attr("href").unwrap()).ok())
  .filter(|candidate| !matches!(candidate.scheme(),"data"|"javascript"))
  .unwrap_or_else(||document_url.clone());
 let mut p=Page{url:url.into(), depth, size:html.len(), html_extracted:Some(true),directive_rules:Some(vec![]),..Default::default()};
 p.title=doc.select(&sel("title")).next().map(|element|element.text().collect::<String>().trim_matches(|character:char|character <= '\u{20}').to_string()).unwrap_or_default();
 for m in doc.select(&sel("meta")) { let name=m.value().attr("name").unwrap_or("").to_ascii_lowercase(); let value=m.value().attr("content").unwrap_or(""); if name=="description" {p.description=value.trim_matches(|character:char|character <= '\u{20}').into()} if let Some(rule)=crate::directives::meta(&name,value){p.directive_rules.as_mut().unwrap().push(rule)} if ["robots","googlebot"].contains(&name.as_str()) {p.directives.push_str(value);p.directives.push(' ')} }
 p.headings=(1..=6).map(|n|doc.select(&sel(&format!("h{n}"))).map(text).collect()).collect();
 for l in doc.select(&sel("link[href]")) {let rel=l.value().attr("rel").unwrap_or(""); if let Some(href)=absolute(&base,l.value().attr("href").unwrap()) {if rel.split_whitespace().any(|r|r.eq_ignore_ascii_case("canonical")){p.canonical=href.clone()} if rel.split_whitespace().any(|r|r.eq_ignore_ascii_case("alternate")){if let Some(lang)=l.value().attr("hreflang"){p.hreflang.push((lang.into(),href))}}} }
 for a in doc.select(&sel("a[href],area[href]")) {if let Some(u)=absolute(&base,a.value().attr("href").unwrap()) {let internal=Url::parse(&u)?.origin()==document_url.origin();p.links.push(Link{url:u,text:text(a),rel:a.value().attr("rel").unwrap_or("").into(),kind:"anchor".into(),internal,..link_metadata(a,a.value().attr("href").unwrap())})}}
 for (selector,attribute,kind) in [("script[src]","src","script"),("link[rel='stylesheet'][href]","href","stylesheet")] {for a in doc.select(&sel(selector)){if let Some(u)=absolute(&base,a.value().attr(attribute).unwrap()){let internal=Url::parse(&u)?.origin()==document_url.origin();p.links.push(Link{url:u,kind:kind.into(),internal,..link_metadata(a,a.value().attr(attribute).unwrap())})}}}
 for i in doc.select(&sel("img[src]")) {if let Some(u)=absolute(&base,i.value().attr("src").unwrap()){p.images.push((u,i.value().attr("alt").unwrap_or("").into()))}}
 p.json_ld=doc.select(&sel("script[type='application/ld+json']")).map(|e|e.inner_html()).collect();
 let body=doc.select(&sel("body")).next().map(visible_text).unwrap_or_default();let content=doc.select(&sel("body")).next().map(content_text).unwrap_or_default();p.word_count=doc.select(&sel("body")).next().map(text_metrics::word_count).unwrap_or(0);p.body_terms=Some(body_terms(&body));if !content.trim().is_empty(){p.content_hash=format!("{:x}",Sha256::digest(content.as_bytes()));}
 if p.title.is_empty(){p.issues.push("Missing title".into())} if p.description.is_empty(){p.issues.push("Missing meta description".into())} if p.headings[0].iter().all(|value|value.trim().is_empty()){p.issues.push("Missing H1".into())} if p.headings[0].len()>1{p.issues.push("Multiple H1".into())} if p.images.iter().any(|(_,a)|a.is_empty()){p.issues.push("Images missing alt text".into())} if p.json_ld.iter().any(|v|serde_json::from_str::<serde_json::Value>(v).is_err()){p.issues.push("Invalid JSON-LD".into())} Ok(p)
}
fn open(db:&str)->Result<Connection>{database::open(db)}
fn open_read(db:&str)->Result<Connection>{database::read(db)}
pub fn project_seed(db:&str)->Result<String>{Ok(open_read(db)?.query_row("SELECT value FROM metadata WHERE key='seed'",[],|r|r.get(0))?)}
pub fn pending_count(db:&str)->Result<usize>{Ok(open_read(db)?.query_row("SELECT COUNT(*) FROM frontier WHERE state IN ('pending','inflight')",[],|r|r.get(0))?)}
pub fn project_mode(db:&str)->Result<String>{Ok(open_read(db)?.query_row("SELECT value FROM metadata WHERE key='mode'",[],|r|r.get(0)).optional()?.unwrap_or_else(||"spider".into()))}
pub fn project_urls(db:&str)->Result<Vec<String>>{let c=open_read(db)?;let mut statement=c.prepare("SELECT url FROM frontier ORDER BY rowid")?;let rows=statement.query_map([],|r|r.get::<_,String>(0))?;Ok(rows.collect::<std::result::Result<Vec<_>,_>>()?)}
/// One read-only SQLite snapshot for project metadata and observations.
#[derive(Debug,Serialize)]pub struct SavedProject{pub pages:Vec<Page>,pub seed:String,pub mode:String,pub profile:CrawlProfile,pub list_urls:Vec<String>,pub queued:usize}
pub fn load_project_data(db:&str)->Result<SavedProject>{
 let mut c=open_read(db)?;let tx=c.transaction()?;
 let seed:String=tx.query_row("SELECT value FROM metadata WHERE key='seed'",[],|row|row.get(0))?;
 let mode:String=tx.query_row("SELECT value FROM metadata WHERE key='mode'",[],|row|row.get(0)).optional()?.unwrap_or_else(||"spider".into());if !["spider","list"].contains(&mode.as_str()){return Err("Saved crawl has an unsupported mode".into())}
 let profile:Option<String>=tx.query_row("SELECT value FROM metadata WHERE key='profile'",[],|row|row.get(0)).optional()?;let profile=match profile{Some(value)=>serde_json::from_str(&value)?,None=>CrawlProfile::default()};
 let mut pages=Vec::new();{let mut statement=tx.prepare("SELECT json FROM pages ORDER BY url")?;for row in statement.query_map([],|row|row.get::<_,String>(0))?{pages.push(serde_json::from_str(&row?)?);}}
 let list_urls=if mode=="list"{let mut statement=tx.prepare("SELECT url FROM frontier ORDER BY rowid")?;let rows=statement.query_map([],|row|row.get::<_,String>(0))?;rows.collect::<std::result::Result<Vec<_>,_>>()?}else{Vec::new()};
 let queued=tx.query_row("SELECT COUNT(*) FROM frontier WHERE state IN ('pending','inflight')",[],|row|row.get(0))?;tx.commit()?;Ok(SavedProject{pages,seed,mode,profile,list_urls,queued})
}
pub fn load_pages(db:&str)->Result<Vec<Page>>{let c=open_read(db)?;let mut s=c.prepare("SELECT json FROM pages ORDER BY url")?; let rows=s.query_map([],|r|r.get::<_,String>(0))?;let mut out=Vec::new();for r in rows{out.push(serde_json::from_str(&r?)?)}Ok(out)}

// Most specific prefix wins; conservative wildcard handling blocks when ambiguous.
pub fn robots_allowed(source:&str,path:&str)->bool{robots_allowed_for(source,path,"FITIGBot")}
pub fn robots_allowed_for(source:&str,path:&str,user_agent:&str)->bool {
 let product=user_agent.split_whitespace().next().unwrap_or("FITIGBot").split('/').next().unwrap_or("FITIGBot").to_ascii_lowercase();
 let mut groups:Vec<(Vec<String>,Vec<(bool,String)>)>=Vec::new();let mut agents=Vec::new();let mut directives=Vec::new();let mut has_directive=false;
 for line in source.lines(){let line=line.split('#').next().unwrap_or("").trim();if let Some((key,value))=line.split_once(':'){let key=key.trim().to_ascii_lowercase();let value=value.trim();if key=="user-agent"{if has_directive{groups.push((std::mem::take(&mut agents),std::mem::take(&mut directives)));has_directive=false}agents.push(value.to_ascii_lowercase())}else if !agents.is_empty(){has_directive=true;if ["allow","disallow"].contains(&key.as_str())&&!value.is_empty(){directives.push((key=="allow",value.into()))}}}}
 if !agents.is_empty(){groups.push((agents,directives))}
 let specificity=|agents:&Vec<String>|agents.iter().filter_map(|a|if a=="*"{Some(0)}else if product.starts_with(a.as_str()){Some(a.len())}else{None}).max();
 let best=groups.iter().filter_map(|(a,_)|specificity(a)).max();if best.is_none(){return true}let mut rules=Vec::new();
 for (agents,directives) in groups{if specificity(&agents)!=best{continue}for (allow,value) in directives{let end=value.ends_with('$');let raw=if end{&value[..value.len()-1]}else{value.as_str()};let pattern=format!("^{}{}",regex::escape(raw).replace("\\*",".*"),if end{"$"}else{""});if regex::Regex::new(&pattern).map(|r|r.is_match(path)).unwrap_or(false){rules.push((raw.replace('*',"").len(),allow))}}}
 rules.sort();rules.last().map(|r|r.1).unwrap_or(true)
}

pub fn crawl(options:CrawlOptions,emit:impl FnMut(Progress))->Result<Vec<Page>>{crawl_impl(options,None,CrawlProfile::default(),None,None,render::render,emit)}
pub fn crawl_list(options:CrawlOptions,urls:Vec<String>,emit:impl FnMut(Progress))->Result<Vec<Page>>{crawl_configured(options,Some(urls),CrawlProfile::default(),emit)}
pub fn crawl_configured(options:CrawlOptions,urls:Option<Vec<String>>,profile:CrawlProfile,emit:impl FnMut(Progress))->Result<Vec<Page>>{crawl_configured_with_renderer(options,urls,profile,render::render,emit)}
pub fn crawl_configured_with_renderer<D:Into<render::RenderedDocument>>(options:CrawlOptions,urls:Option<Vec<String>>,profile:CrawlProfile,renderer:impl Fn(&str,&render::Rendering)->Result<D>,emit:impl FnMut(Progress))->Result<Vec<Page>>{crawl_with_authentication_and_renderer(options,urls,profile,None,renderer,emit)}
pub fn crawl_with_authentication_and_renderer<D:Into<render::RenderedDocument>>(options:CrawlOptions,urls:Option<Vec<String>>,profile:CrawlProfile,authentication:Option<authentication::Session>,renderer:impl Fn(&str,&render::Rendering)->Result<D>,emit:impl FnMut(Progress))->Result<Vec<Page>>{crawl_with_transport_and_renderer(options,urls,profile,authentication,None,renderer,emit)}
pub fn crawl_with_transport_and_renderer<D:Into<render::RenderedDocument>>(mut options:CrawlOptions,urls:Option<Vec<String>>,profile:CrawlProfile,authentication:Option<authentication::Session>,proxy_authentication:Option<authentication::Session>,renderer:impl Fn(&str,&render::Rendering)->Result<D>,emit:impl FnMut(Progress))->Result<Vec<Page>>{
 if urls.is_none(){return crawl_impl(options,None,profile,authentication,proxy_authentication,renderer,emit)}
 let urls=urls.unwrap();
 let mut normalized=Vec::new();let mut seen=std::collections::HashSet::new();
 for raw in urls {let mut url=Url::parse(raw.trim())?;if !["http","https"].contains(&url.scheme())||url.host_str().is_none(){return Err("List mode requires absolute HTTP(S) URLs".into())}url.set_fragment(None);let url=url.to_string();if seen.insert(url.clone()){normalized.push(url)}}
 options.url=normalized.first().ok_or("The URL list is empty")?.clone();crawl_impl(options,Some(normalized),profile,authentication,proxy_authentication,renderer,emit)
}

fn crawl_impl<D:Into<render::RenderedDocument>>(options:CrawlOptions,list:Option<Vec<String>>,profile:CrawlProfile,authentication:Option<authentication::Session>,proxy_authentication:Option<authentication::Session>,renderer:impl Fn(&str,&render::Rendering)->Result<D>,mut emit:impl FnMut(Progress))->Result<Vec<Page>>{
 proxy_settings::validate(&profile,proxy_authentication.as_ref())?;
 authentication::validate_configuration(&profile.http_auth_origin,authentication.as_ref(),profile.rendering.enabled)?;
 http_cookies::validate(profile.cookies_enabled,profile.rendering.enabled)?;
 let program=CustomProgram::compile(&profile.custom_rules)?;render::validate(&profile.rendering)?;let rewrite=rewriting::Program::compile(&profile.url_rewriting)?;
 validate_request_timeout(profile.request_timeout_secs)?;
 if !(1..=32).contains(&profile.concurrency){return Err("Concurrency must be between 1 and 32".into())}
 if profile.user_agent.trim().is_empty()||profile.user_agent.len()>1024||profile.user_agent.contains(['\r','\n']){return Err("Supply a valid user agent (1–1024 bytes)".into())}
 if profile.include_patterns.len()>100||profile.exclude_patterns.len()>100||profile.include_patterns.iter().chain(&profile.exclude_patterns).any(|s|s.len()>8192){return Err("Crawl scope supports at most 100 include/exclude expressions, 8192 bytes each".into())}
 let includes=regex::RegexSet::new(&profile.include_patterns)?;let excludes=regex::RegexSet::new(&profile.exclude_patterns)?;
 let seed=Url::parse(&options.url)?;if !["http","https"].contains(&seed.scheme()) || seed.host_str().is_none(){return Err("Seed must be an absolute HTTP(S) URL".into())}
 let scope=scope::Scope::new(&seed,&profile.subdomain_root)?;
 let mode=if list.is_some(){"list"}else{"spider"};
 // Validate saved state through a read-only connection before any migration or write.
 if options.resume{
  let saved=open_read(&options.db)?;
  let stored:String=saved.query_row("SELECT value FROM metadata WHERE key='seed'",[],|r|r.get(0))?;
  if stored!=seed.as_str(){return Err("Resume seed differs from saved crawl".into())}
  if project_mode(&options.db)?!=mode{return Err("Resume mode differs from saved crawl".into())}
  if let Some(urls)=&list{if project_urls(&options.db)?!=*urls{return Err("Resume URL list differs from saved crawl".into())}}
  if project_profile(&options.db)?!=profile{return Err("Resume profile differs from saved crawl; restore its custom/rendering settings first".into())}
 }else if std::path::Path::new(&options.db).exists(){return Err("A project already exists at this path; resume it or choose a new project path".into())}
 let cookie_jar=if profile.cookies_enabled{Some(if options.resume{http_cookies::Jar::restore(&options.db)?.unwrap_or(std::sync::Arc::new(http_cookies::Jar::new()?))}else{std::sync::Arc::new(http_cookies::Jar::new()?)})}else{None};
 let mut c=open(&options.db)?;
 if !options.resume{
  let transaction=c.transaction()?;
  transaction.execute("INSERT INTO metadata VALUES('seed',?1)",[seed.as_str()])?;
  transaction.execute("INSERT INTO metadata VALUES('mode',?1)",[mode])?;
  transaction.execute("INSERT INTO metadata VALUES('profile',?1)",[serde_json::to_string(&profile)?])?;
  transaction.execute("INSERT INTO frontier(url,depth) VALUES(?1,0)",[seed.as_str()])?;
  if let Some(urls)=&list{for url in urls{transaction.execute("INSERT OR IGNORE INTO frontier(url,depth) VALUES(?1,0)",[url])?;}}
  transaction.commit()?;
 }
 c.execute("UPDATE frontier SET state='pending' WHERE state='inflight'",[])?;
 let mut scheduler=http_crawl::Scheduler::new(&seed,&profile,authentication,proxy_authentication,cookie_jar.clone())?;
 let stop=std::path::PathBuf::from(format!("{}.stop",options.db));
 let mut completed=load_pages(&options.db)?.len();
 while options.limit==0 || completed<options.limit {
  let fetched=match scheduler.next(&c,completed,options.limit,options.delay_ms,&stop)?{Some(job)=>job,None=>{if stop.exists(){emit(Progress{event:"stopped".into(),completed,queued:pending_count(&options.db)?,page:None});}break}};
  let url=fetched.url;let depth=fetched.depth;let parsed=Url::parse(&url)?;
  let mut p=Page{url:url.clone(),depth,internal:scope.contains(&parsed),subdomain_root:profile.subdomain_root.clone(),elapsed_ms:fetched.elapsed_ms,..Default::default()};let mut source=None;let mut rendered_source=None;let mut screenshot=None;let mut console=None;let mut javascript=None;let mut accessibility=None;let mut mobile=None;let start=Instant::now();
  if !fetched.error.is_empty(){p.error=fetched.error;p.issues.push(if fetched.blocked{p.error.clone()}else{"Request failed".into()})}else{
    p.status=fetched.status;p.headers=fetched.headers;if profile.cookies_enabled&&p.headers.contains_key("set-cookie"){p.headers.insert("set-cookie".into(),"[redacted: values protected in HTTP cookie checkpoint]".into());}p.content_type=p.headers.get("content-type").cloned().unwrap_or_default();
    if (300..400).contains(&p.status){p.redirect=p.headers.get("location").and_then(|v|absolute(&parsed,v)).unwrap_or_default()}
    let body=fetched.body;p.size=body.len();p.response_hash=response_body_hash(&body);
    if body.len()>10*1024*1024{p.html_extracted=Some(false);p.issues.push("Body exceeds 10 MiB limit; extraction skipped".into())}else if (p.internal||list.is_some())&&p.content_type.contains("html"){let decoded=encoding::decode_html(&body,&p.content_type);p.charset=decoded.charset;p.charset_source=decoded.source.into();p.decoding_errors=decoded.had_errors;let raw=decoded.text;p.source_stored=profile.store_html;if profile.store_html{source=Some(raw.clone())}
     let html=if profile.rendering.enabled&&(200..300).contains(&p.status){match renderer(&url,&profile.rendering).and_then(|document|document.into().validated(&url)){Ok(document)=>{if let Some(report)=&document.mobile{p.mobile_findings=report.total_findings;p.mobile_error=report.error.clone();p.mobile_truncated=report.truncated;}mobile=document.mobile;if let Some(report)=&document.accessibility{p.accessibility_violations=report.counts.as_ref().map(|counts|counts.violations);p.accessibility_nodes=report.counts.as_ref().map(|counts|counts.violation_nodes);p.accessibility_review=report.counts.as_ref().map(|counts|counts.incomplete);p.accessibility_error=report.error.clone();p.accessibility_truncated=report.truncated;}accessibility=document.accessibility;javascript=document.javascript;if let Some(report)=&javascript{p.javascript_count=Some(report.results.len());p.javascript_errors=Some(report.results.iter().filter(|entry|!entry.error.is_empty()).count());}console=document.console;if let Some(report)=&console{p.console_count=Some(report.entries.len());p.console_errors=Some(report.entries.iter().filter(|entry|matches!(entry.level.as_str(),"Error"|"Exception"|"Assert")).count());p.console_error=report.error.clone();}screenshot=document.screenshot_png;p.screenshot_error=document.screenshot_error;p.screenshot_stored=screenshot.is_some();p.rendered=true;p.rendered_url=document.url;if profile.store_html{rendered_source=Some(document.html.clone())}document.html},Err(error)=>{p.render_error=error.to_string();raw}}}else{raw};
     p.custom=program.run(&html);let extracted=if (200..300).contains(&p.status){extract(if p.rendered_url.is_empty(){&url}else{&p.rendered_url},&html,depth)?}else{Page::default()};p.html_extracted=Some((200..300).contains(&p.status));p.title=extracted.title;p.description=extracted.description;p.headings=extracted.headings;p.canonical=extracted.canonical;p.directives=extracted.directives;p.directive_rules=extracted.directive_rules;if p.rendered{if let Some(rules)=p.directive_rules.as_mut(){for rule in rules{rule.source="Rendered HTML meta".into()}}}p.hreflang=extracted.hreflang;p.links=extracted.links;for link in &mut p.links{link.internal=scope.contains(&Url::parse(&link.url)?)}p.images=extracted.images;p.json_ld=extracted.json_ld;p.word_count=extracted.word_count;p.body_terms=extracted.body_terms;p.content_hash=extracted.content_hash;p.issues=extracted.issues;}else if (p.internal||list.is_some())&&p.content_type.contains("xml"){
     let xml=String::from_utf8_lossy(&body);let locations=regex::Regex::new(r"(?is)<loc>\s*([^<]+)\s*</loc>")?;for loc in locations.captures_iter(&xml){if let Some(u)=absolute(&parsed,&loc[1].trim().replace("&amp;","&")){let internal=scope.contains(&Url::parse(&u)?);p.links.push(Link{url:u,text:String::new(),rel:String::new(),kind:"sitemap".into(),internal,..Default::default()})}}
    }
    if p.decoding_errors{p.issues.push("HTML decoding replaced invalid byte sequences".into())}if !p.render_error.is_empty(){p.issues.push("JavaScript rendering failed; extracted HTTP source instead".into())}if p.status>=400{p.issues.push("HTTP error".into())}if parsed.scheme()=="http"{p.issues.push("HTTP URL".into())}if p.status>0{let rules=p.directive_rules.get_or_insert_with(Vec::new);if let Some(v)=p.headers.get("x-robots-tag"){rules.extend(crate::directives::headers(v));p.directives.push_str(v)}}
  }
  if stop.exists(){scheduler.cancel(&c)?;break}
  p.elapsed_ms+=start.elapsed().as_millis();if p.console_errors.unwrap_or(0)>0{p.issues.push("JavaScript console errors or exceptions captured".into())}if !p.console_error.is_empty(){p.issues.push("Browser console capture incomplete".into())}if !p.screenshot_error.is_empty(){p.issues.push("Rendered screenshot capture failed".into())}if p.javascript_errors.unwrap_or(0)>0{p.issues.push("Custom JavaScript execution errors".into())}if p.accessibility_violations.unwrap_or(0)>0{p.issues.push("Accessibility violations recorded".into())}if p.accessibility_review.unwrap_or(0)>0{p.issues.push("Accessibility checks need manual review".into())}if !p.accessibility_error.is_empty(){p.issues.push("Accessibility audit unavailable".into())}if p.accessibility_truncated{p.issues.push("Accessibility finding details truncated".into())}if p.mobile_findings.unwrap_or(0)>0{p.issues.push("Mobile layout observations require review".into())}if !p.mobile_error.is_empty(){p.issues.push("Mobile audit unavailable".into())}if p.mobile_truncated{p.issues.push("Mobile finding details truncated".into())}let tx=c.transaction()?;if let Some(report)=mobile{crate::mobile::save(&tx,&url,&report)?;}if let Some(report)=accessibility{crate::accessibility::save(&tx,&url,&report)?;}if let Some(report)=javascript{crate::javascript::save(&tx,&url,&report)?;}if let Some(report)=console{tx.execute("INSERT OR REPLACE INTO browser_logs VALUES(?1,?2)",params![url,serde_json::to_string(&report)?])?;}if let Some(png)=screenshot{tx.execute("INSERT OR REPLACE INTO screenshots VALUES(?1,?2)",params![url,png])?;}if let Some(html)=rendered_source{tx.execute("INSERT OR REPLACE INTO rendered_documents VALUES(?1,?2)",params![url,html])?;}if let Some(html)=source{tx.execute("INSERT OR REPLACE INTO documents VALUES(?1,?2)",params![url,html])?;}tx.execute("INSERT OR REPLACE INTO pages VALUES(?1,?2)",params![url,serde_json::to_string(&p)?])?;tx.execute("UPDATE frontier SET state='done' WHERE url=?1",[&url])?;
  let mut discovered:Vec<String>=if rewrite.enabled(){if p.internal{p.links.iter().map(|link|link.url.clone()).chain(p.images.iter().map(|(url,_)|url.clone())).chain((!p.redirect.is_empty()).then(||p.redirect.clone())).collect()}else{vec![]}}else{p.links.iter().filter(|l|p.internal&&(l.internal||profile.check_external)).map(|l|l.url.clone()).collect()};
  if !rewrite.enabled(){for (image,_) in &p.images{if profile.check_external||scope.contains(&Url::parse(image)?){discovered.push(image.clone())}}if p.internal&&!p.redirect.is_empty(){if profile.check_external||scope.contains(&Url::parse(&p.redirect)?){discovered.push(p.redirect.clone())}}}
  let mut rewrite_report=if rewrite.enabled()&&p.internal{Some(rewriting::Report::default())}else{None};let mut seen=std::collections::HashSet::new();
  for original in discovered{if !seen.insert(original.clone()){continue}let target=match rewrite.apply(&original){Ok(value)=>value,Err(error)=>{if let Some(report)=&mut rewrite_report{report.record(rewriting::Entry{original_url:original,crawl_url:None,disposition:"Invalid: not queued".into(),error:error.to_string()})}continue}};
   let disposition=if !scope.contains(&Url::parse(&target)?)&&!profile.check_external{"Outside scope: not queued"}else if list.is_some(){"List mode: not queued"}else if profile.max_depth!=0&&depth>=profile.max_depth{"Depth limit: not queued"}else if !(includes.is_empty()||includes.is_match(&target))||excludes.is_match(&target){"URL filter: not queued"}else{"Eligible for queue"};
   if let Some(report)=&mut rewrite_report{if target!=original{report.record(rewriting::Entry{original_url:original,crawl_url:Some(target.clone()),disposition:disposition.into(),error:String::new()})}}
   if disposition=="Eligible for queue"{tx.execute("INSERT OR IGNORE INTO frontier(url,depth) VALUES(?1,?2)",params![target,depth+1])?;}
  }
  if let Some(report)=rewrite_report{p.rewrite_count=Some(report.changed);p.rewrite_errors=Some(report.errors);if report.errors>0{p.issues.push("URL rewriting errors; invalid targets not queued".into())}rewriting::save(&tx,&url,&report)?;tx.execute("UPDATE pages SET json=?1 WHERE url=?2",params![serde_json::to_string(&p)?,url])?;}
  if let Some(jar)=&cookie_jar{jar.checkpoint(&tx)?;}tx.commit()?;completed+=1;let queued=c.query_row("SELECT COUNT(*) FROM frontier WHERE state IN ('pending','inflight')",[],|r|r.get(0))?;emit(Progress{event:"page".into(),completed,queued,page:Some(p)});
 }
 emit(Progress{event:"complete".into(),completed,queued:c.query_row("SELECT COUNT(*) FROM frontier WHERE state IN ('pending','inflight')",[],|r|r.get(0))?,page:None});load_pages(&options.db)
}

/// SHA-256 of the complete received payload, before character decoding.
/// Oversized bodies are truncated by the fetcher and must remain unknown.
fn response_body_hash(body:&[u8])->Option<String>{
 if body.len()>10*1024*1024 {None}else{Some(format!("{:x}",Sha256::digest(body)))}
}
#[cfg(test)]mod response_hash_tests{
 use super::*;
 #[test]fn exact_bytes_empty_and_truncation(){
  assert_eq!(response_body_hash(b"" ).unwrap(),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
  assert_eq!(response_body_hash(b"abc").unwrap(),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  assert_ne!(response_body_hash(b"abc"),response_body_hash(b"abc "));
  assert!(response_body_hash(&vec![0;10*1024*1024+1]).is_none());
  let legacy:Page=serde_json::from_str(&serde_json::to_string(&Page::default()).unwrap().replace("\"response_hash\":null,", "")).unwrap();assert!(legacy.response_hash.is_none());
 }
}
pub fn analyze(db:&str)->Result<serde_json::Value>{let pages=load_pages(db)?;let by_url:BTreeMap<_,_>=pages.iter().map(|p|(p.url.as_str(),p)).collect();let mut duplicates:BTreeMap<String,Vec<String>>=BTreeMap::new();let mut response_duplicates:BTreeMap<String,Vec<String>>=BTreeMap::new();let mut inlinks:BTreeMap<String,Vec<String>>=BTreeMap::new();let mut broken=Vec::new();for p in &pages{if let Some(hash)=&p.response_hash{response_duplicates.entry(hash.clone()).or_default().push(p.url.clone());}if !p.content_hash.is_empty(){duplicates.entry(p.content_hash.clone()).or_default().push(p.url.clone())}for l in &p.links{inlinks.entry(l.url.clone()).or_default().push(p.url.clone());if let Some(target)=by_url.get(l.url.as_str()){if target.status>=400{broken.push(serde_json::json!({"source":p.url,"target":l.url,"status":target.status}))}}}}duplicates.retain(|_,v|v.len()>1);response_duplicates.retain(|_,v|v.len()>1);Ok(serde_json::json!({"pages":pages.len(),"duplicate_content":duplicates,"duplicate_response_bodies":response_duplicates,"inlinks":inlinks,"broken_links":broken,"post_crawl":diagnostics::analyze(&pages)}))}
pub fn compare(db:&str,other:&str)->Result<serde_json::Value>{let old=load_pages(db)?;let new=load_pages(other)?;let a:BTreeMap<_,_>=old.iter().map(|p|(&p.url,p)).collect();let b:BTreeMap<_,_>=new.iter().map(|p|(&p.url,p)).collect();let mut changed=Vec::new();for (u,p) in &b{if let Some(q)=a.get(u){let mut fields=Vec::new();if p.status!=q.status{fields.push("status")}if p.title!=q.title{fields.push("title")}if p.description!=q.description{fields.push("description")}if p.canonical!=q.canonical{fields.push("canonical")}if p.content_hash!=q.content_hash{fields.push("content")}if let(Some(a),Some(b))=(&p.response_hash,&q.response_hash){if a!=b{fields.push("response_hash")}}if !fields.is_empty(){changed.push(serde_json::json!({"url":u,"fields":fields}))}}}Ok(serde_json::json!({"added":b.keys().filter(|u|!a.contains_key(*u)).collect::<Vec<_>>(),"removed":a.keys().filter(|u|!b.contains_key(*u)).collect::<Vec<_>>(),"changed":changed}))}
fn escape(v:&str)->String{v.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;")}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct LinkReportRow {pub link_type:String,pub from:String,pub to:String,pub anchor_text:String,pub alt_text:String,pub follow:Option<bool>,pub rel:String,pub target:String,pub status:Option<u16>,pub position:String,pub path_type:String,pub origin:String,pub internal:bool}
pub fn export_link_rows(rows:&[LinkReportRow],out:&str)->Result<()>{export_target::check(out)?;use std::io::Write;let mut file=std::fs::File::create(out)?;file.write_all(b"\xEF\xBB\xBF")?;let mut writer=csv::Writer::from_writer(file);writer.write_record(["Type","From","To","Anchor Text","Alt Text","Follow","Rel","Target","Status Code","Link Position","Path Type","Link Origin","Internal","From (Decoded)","To (Decoded)"])?;for row in rows{writer.write_record([row.link_type.clone(),row.from.clone(),row.to.clone(),row.anchor_text.clone(),row.alt_text.clone(),row.follow.map(|v|v.to_string()).unwrap_or_default(),row.rel.clone(),row.target.clone(),row.status.map(|v|v.to_string()).unwrap_or_default(),row.position.clone(),row.path_type.clone(),row.origin.clone(),row.internal.to_string(),display_url(&row.from),display_url(&row.to)])?;}writer.flush()?;Ok(())}
pub fn export_sitemap(db:&str,out:&str)->Result<usize>{let pages=load_pages(db)?;let eligible=pages.iter().filter(|p|p.internal&&p.html_extracted!=Some(false)&&(200..300).contains(&p.status)&&p.content_type.to_ascii_lowercase().contains("html")&&!crate::directives::restriction(p,"googlebot","noindex")&&(p.canonical.is_empty()||p.canonical==p.url)).collect::<Vec<_>>();if eligible.len()>50_000{return Err("Use the sitemap ZIP package export for more than 50,000 eligible URLs".into())}let mut xml=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");for page in &eligible{xml.push_str(&format!("  <url><loc>{}</loc></url>\n",escape(&display_url(&page.url))))}xml.push_str("</urlset>\n");std::fs::write(out,xml)?;Ok(eligible.len())}
#[cfg(feature="xlsx")]
pub fn export_xlsx(db:&str,out:&str)->Result<()>{
 let pages=load_pages(db)?;if pages.len()>1_048_575{return Err("Excel row limit exceeded".into())}let mut workbook=rust_xlsxwriter::Workbook::new();let sheet=workbook.add_worksheet();sheet.set_name("Crawl")?;
 for (col,title) in ["URL","Status","Title","Description","Canonical","Depth","Words","Issues","URL (Decoded)"].iter().enumerate(){sheet.write_string(0,col as u16,*title)?;}
 for (index,p) in pages.iter().enumerate(){let row=(index+1)as u32;sheet.write_string(row,0,&p.url)?;sheet.write_number(row,1,p.status as f64)?;sheet.write_string(row,2,&p.title)?;sheet.write_string(row,3,&p.description)?;sheet.write_string(row,4,&p.canonical)?;sheet.write_number(row,5,p.depth as f64)?;sheet.write_number(row,6,p.word_count as f64)?;sheet.write_string(row,7,p.issues.join("; "))?;sheet.write_string(row,8,display_url(&p.url))?;}workbook.save(out)?;Ok(())
}
#[cfg(not(feature="xlsx"))]
pub fn export_xlsx(_db:&str,_out:&str)->Result<()>{Err("XLSX support disabled; build with feature xlsx".into())}
pub fn export(db:&str,out:&str,format:&str)->Result<()>{export_target::check(out)?;if format=="sitemap"{export_sitemap(db,out)?;return Ok(())}if format=="xlsx"{return export_xlsx(db,out)}let pages=load_pages(db)?;match format{"csv"=>{let mut file=std::fs::File::create(out)?;use std::io::Write;file.write_all(b"\xEF\xBB\xBF")?;let mut w=csv::Writer::from_writer(file);w.write_record(["URL","Status","Title","Description","Canonical","Depth","Words","Issues","URL (Decoded)"])?;for p in pages{let decoded=display_url(&p.url);w.write_record([p.url,p.status.to_string(),p.title,p.description,p.canonical,p.depth.to_string(),p.word_count.to_string(),p.issues.join("; "),decoded])?}w.flush()?},"html"=>{let mut h=String::from("<!doctype html><meta charset=utf-8><title>FITIG crawl report</title><style>body{font:14px system-ui}table{border-collapse:collapse}td,th{border:1px solid #aaa;padding:6px}</style><h1>FITIG crawl report</h1><table><tr><th>URL<th>Status<th>Title<th>Issues");for p in pages{h.push_str(&format!("<tr><td>{}<td>{}<td>{}<td>{}",escape(&display_url(&p.url)),p.status,escape(&p.title),escape(&p.issues.join("; "))))}h.push_str("</table>");std::fs::write(out,h)?},_=>return Err("Format must be csv, html or xlsx".into())}Ok(())}

#[cfg(test)]mod tests{use super::*;
 #[test]fn link_metadata_and_old_project_compatibility(){let p=extract("https://example.test/","<nav><a href='/a' target='_blank' rel='nofollow'><img src='/logo.png' alt='לוגו'>שלום</a></nav><footer><a href='https://off.test/'>off</a></footer>",0).unwrap();assert_eq!(p.links[0].alt,"לוגו");assert_eq!(p.links[0].target,"_blank");assert_eq!(p.links[0].position,"Navigation");assert_eq!(p.links[0].path_type,"Root-relative");assert_eq!(p.links[1].position,"Footer");assert_eq!(p.links[1].path_type,"Absolute");let old:Link=serde_json::from_str(r#"{"url":"https://example.test/","text":"old","rel":"","kind":"anchor","internal":true}"#).unwrap();assert_eq!(old.text,"old");assert_eq!(old.position,"");}
 #[test]fn filtered_link_export_keeps_order_types_and_hebrew(){let path=std::env::temp_dir().join(format!("fitig-links-{}.csv",std::process::id()));let rows=vec![LinkReportRow{link_type:"Hyperlink".into(),from:"https://a.test/".into(),to:"https://a.test/%D7%A9%D7%9C%D7%95%D7%9D".into(),anchor_text:"שלום".into(),follow:Some(false),status:Some(404),..Default::default()},LinkReportRow{link_type:"Image".into(),from:"https://b.test/".into(),to:"https://b.test/i.png".into(),follow:None,..Default::default()}];export_link_rows(&rows,path.to_str().unwrap()).unwrap();assert!(std::fs::read(&path).unwrap().starts_with(b"\xEF\xBB\xBF"));let mut reader=csv::Reader::from_path(&path).unwrap();let actual=reader.records().collect::<std::result::Result<Vec<_>,_>>().unwrap();assert_eq!(actual.len(),2);assert_eq!(&actual[0][1],"https://a.test/");assert_eq!(&actual[0][3],"שלום");assert_eq!(&actual[0][5],"false");assert_eq!(&actual[0][8],"404");assert_eq!(&actual[0][14],"https://a.test/שלום");assert_eq!(&actual[1][5],"");std::fs::remove_file(path).unwrap();}
 #[test]fn sitemap_exports_only_eligible_html(){let folder=std::env::temp_dir().join(format!("fitig-sitemap-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.sqlite");{let connection=open(db.to_str().unwrap()).unwrap();for p in [Page{url:"https://e.test/?a=1&b=2".into(),status:200,content_type:"text/html".into(),..Default::default()},Page{url:"https://e.test/no".into(),status:200,content_type:"text/html".into(),directives:"noindex, follow".into(),..Default::default()},Page{url:"https://e.test/error".into(),status:404,content_type:"text/html".into(),..Default::default()},Page{url:"https://e.test/image".into(),status:200,content_type:"image/png".into(),..Default::default()},Page{url:"https://e.test/canonical".into(),canonical:"https://e.test/other".into(),status:200,content_type:"text/html".into(),..Default::default()}]{connection.execute("INSERT INTO pages VALUES(?1,?2)",params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}}let output=folder.join("sitemap.xml");assert_eq!(export_sitemap(db.to_str().unwrap(),output.to_str().unwrap()).unwrap(),1);let xml=std::fs::read_to_string(output).unwrap();assert!(xml.contains("https://e.test/?a=1&amp;b=2"));assert!(!xml.contains("https://e.test/no"));std::fs::remove_dir_all(folder).unwrap();}
 #[test]fn hebrew_url_display_keeps_request_identity(){let encoded="https://www.fizzio.co.il/%D7%9B%D7%95%D7%A8%D7%A1%D7%AA-%D7%98%D7%9C%D7%95%D7%95%D7%99%D7%96%D7%99%D7%94";assert_eq!(display_url(encoded),"https://www.fizzio.co.il/כורסת-טלוויזיה");assert_eq!(display_url("https://e.test/%D7%A9%2F%D7%9C?x=%26"),"https://e.test/ש%2Fל?x=%26");assert_eq!(display_url("https://e.test/%FF"),"https://e.test/%FF");assert_eq!(display_url("https://e.test/%E2%80%AE"),"https://e.test/%E2%80%AE");let p=extract(encoded,"<a href='/שלום'>שלום</a>",0).unwrap();assert_eq!(p.url,encoded);assert!(p.links[0].url.contains("%D7"));}
 #[test]fn csv_and_html_preserve_hebrew_and_original_urls(){let folder=std::env::temp_dir().join(format!("fitig-hebrew-report-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.sqlite");let encoded="https://example.test/%D7%A9%D7%9C%D7%95%D7%9D";{let c=open(db.to_str().unwrap()).unwrap();let p=Page{url:encoded.into(),title:"שלום עולם".into(),status:200,..Default::default()};c.execute("INSERT INTO pages VALUES(?1,?2)",params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}let csv=folder.join("report.csv");export(db.to_str().unwrap(),csv.to_str().unwrap(),"csv").unwrap();assert!(std::fs::read(&csv).unwrap().starts_with(b"\xEF\xBB\xBF"));let mut reader=csv::Reader::from_path(&csv).unwrap();let record=reader.records().next().unwrap().unwrap();assert_eq!(&record[0],encoded);assert_eq!(&record[2],"שלום עולם");assert_eq!(&record[8],"https://example.test/שלום");let html=folder.join("report.html");export(db.to_str().unwrap(),html.to_str().unwrap(),"html").unwrap();let content=std::fs::read_to_string(html).unwrap();assert!(content.contains("https://example.test/שלום"));assert!(content.contains("שלום עולם"));std::fs::remove_dir_all(folder).unwrap();}
 #[test]fn unlimited_crawl_exceeds_old_500_page_default(){
  use std::io::{Read,Write};use std::net::TcpListener;use std::sync::{Arc,atomic::{AtomicBool,Ordering}};
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());let done=Arc::new(AtomicBool::new(false));let signal=done.clone();
  let server=std::thread::spawn(move||{while !signal.load(Ordering::Relaxed){match listener.accept(){Ok((mut stream,_))=>{stream.set_nonblocking(false).unwrap();stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();let mut request=Vec::new();let mut byte=[0;1];while request.len()<32768&&!request.ends_with(b"\r\n\r\n"){match stream.read(&mut byte){Ok(1)=>request.push(byte[0]),_=>break}}let request=String::from_utf8_lossy(&request);let path=request.split_whitespace().nth(1).unwrap_or("");let body=if path=="/robots.txt"{String::new()}else if path=="/"{format!("<title>Root</title><body>{}</body>",(0..501).map(|i|format!("<a href='/p/{i}'>page</a>")).collect::<String>())}else{"<title>Leaf</title><body><h1>Leaf</h1></body>".into()};let response=format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());let _=stream.write_all(response.as_bytes());},Err(_)=>std::thread::sleep(Duration::from_millis(1))}}});
  let folder=std::env::temp_dir().join(format!("fitig-unlimited-{}-{}",std::process::id(),origin.rsplit(':').next().unwrap()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.sqlite").to_string_lossy().into_owned();let result=crawl(CrawlOptions{url:format!("{origin}/"),db:db.clone(),limit:0,delay_ms:0,resume:false},|_|{});done.store(true,Ordering::Relaxed);server.join().unwrap();let pages=result.unwrap();assert_eq!(pages.len(),502);assert!(pages.iter().all(|p|p.status==200));assert_eq!(pending_count(&db).unwrap(),0);std::fs::remove_dir_all(folder).unwrap();
 }
 #[test]fn retry_get_recovers_from_reset(){let mut attempts=0;let mut waits=Vec::new();let result=retry_get(||{attempts+=1;if attempts<3{Err(std::io::ErrorKind::ConnectionReset)}else{Ok("response")}},|e|transient_io(*e),|delay|waits.push(delay));assert_eq!(result,Ok("response"));assert_eq!(attempts,3);assert_eq!(waits,vec![Duration::from_millis(100),Duration::from_millis(200)]);}
 #[test]fn retry_get_stops_after_three_failures(){let mut attempts=0;let result:std::result::Result<(),_>=retry_get(||{attempts+=1;Err(std::io::ErrorKind::ConnectionReset)},|e|transient_io(*e),|_|{});assert_eq!(result,Err(std::io::ErrorKind::ConnectionReset));assert_eq!(attempts,3);}
 #[test]fn retry_get_does_not_retry_permission_errors(){let mut attempts=0;let result:std::result::Result<(),_>=retry_get(||{attempts+=1;Err(std::io::ErrorKind::PermissionDenied)},|e|transient_io(*e),|_|panic!("Permission errors must not be retried"));assert_eq!(result,Err(std::io::ErrorKind::PermissionDenied));assert_eq!(attempts,1);}
 #[test]fn extraction(){let p=extract("https://example.test/a",r#"<html><head><title>שלום עולם</title><meta name="description" content="hello"><link rel="canonical" href="/c"><link rel="alternate" hreflang="he" href="/he"></head><body><h1>Main</h1><h2>Sub</h2><a href="/b#x">next</a><a href="https://other.test">off</a><img src="/i.png"><script type="application/ld+json">{bad}</script></body></html>"#,2).unwrap();assert_eq!(p.title,"שלום עולם");assert_eq!(p.canonical,"https://example.test/c");assert_eq!(p.links[0].url,"https://example.test/b");assert!(!p.links[1].internal);assert_eq!(p.headings[1],vec!["Sub"]);assert!(p.issues.contains(&"Invalid JSON-LD".into()));assert_eq!(p.depth,2)}
 #[test]fn robots(){assert!(!robots_allowed("User-agent: *\nDisallow: /private\nAllow: /private/public", "/private/a"));assert!(robots_allowed("User-agent: *\nDisallow: /private\nAllow: /private/public", "/private/public/a"));assert!(!robots_allowed("User-agent: *\nDisallow: /*?", "/a?x"));}
 #[test]fn schemes(){let p=extract("https://example.test", "<a href='javascript:alert(1)'>x</a><a href='mailto:a@b'>m</a>",0).unwrap();assert!(p.links.is_empty())}
 #[test]fn escaping(){assert_eq!(escape("<script>\"&"),"&lt;script&gt;&quot;&amp;")}
 #[test]fn robots_groups_end_anchor_and_tie(){
  let rules="User-agent: *\nDisallow: /\nUser-agent: Other\nUser-agent: FITIGBot\nDisallow: /private$\nAllow: /private/public\nUser-agent: FITIGBot\nDisallow: /second";
  assert!(robots_allowed(rules,"/public"));assert!(!robots_allowed(rules,"/private"));assert!(robots_allowed(rules,"/private/a"));assert!(!robots_allowed(rules,"/second"));assert!(robots_allowed("User-agent: *\nDisallow: /a\nAllow: /a","/a"));assert!(robots_allowed("User-agent: OtherBot\nDisallow: /","/"));
 }
 #[test]fn origin_scope_and_empty_hash(){let p=extract("https://example.test", "<body><a href='https://example.test:8443/a'>port</a><a href='http://example.test/a'>protocol</a><a href='/same'>same</a></body>",0).unwrap();assert!(!p.links[0].internal);assert!(!p.links[1].internal);assert!(p.links[2].internal);assert!(extract("https://example.test","<body><script>code words</script><style>css words</style> </body>",0).unwrap().content_hash.is_empty());}
 #[test]fn base_resolves_metadata_assets_and_hebrew(){
  let page=extract("https://example.test/old/page",r#"<base href="/קטלוג/"><base href="/ignored/"><link rel="canonical" href="item"><link rel="alternate" hreflang="he" href="עברית"><a href="item#section">item</a><script src="app.js"></script><link rel="stylesheet" href="main.css"><img src="photo.png" alt="צילום">"#,0).unwrap();
  let root=Url::parse("https://example.test/קטלוג/").unwrap();
  assert_eq!(page.url,"https://example.test/old/page");
  assert_eq!(page.canonical,root.join("item").unwrap().as_str());
  assert_eq!(page.hreflang[0].1,root.join("עברית").unwrap().as_str());
  assert_eq!(page.links.len(),3);assert!(page.links.iter().all(|link|link.internal));
  assert_eq!(page.links[0].url,page.canonical);
  assert_eq!(page.links[1].url,root.join("app.js").unwrap().as_str());
  assert_eq!(page.links[2].url,root.join("main.css").unwrap().as_str());
  assert_eq!(page.images[0].0,root.join("photo.png").unwrap().as_str());
 }
 #[test]fn external_base_does_not_reclassify_external_links_as_internal(){
  let page=extract("https://example.test/",r#"<base href="https://cdn.test/assets/"><a href="page">external</a><script src="app.js"></script><a href="https://example.test/home">internal</a>"#,0).unwrap();
  assert!(!page.links[0].internal);assert!(page.links[1].internal);assert!(!page.links[2].internal);
  assert_eq!(page.links[0].url,"https://cdn.test/assets/page");
 }
 #[test]fn first_invalid_base_falls_back_without_using_second(){
  for value in ["http://[invalid", "javascript:alert(1)","data:text/html,x"] {
   let page=extract("https://example.test/folder/page",&format!("<base href='{value}'><base href='/wrong/'><a href='next'>next</a>"),0).unwrap();
   assert_eq!(page.links[0].url,"https://example.test/folder/next");
  }
  let page=extract("https://example.test/folder/page","<base target='_blank'><base href='../right/'><a href='next'>next</a>",0).unwrap();
  assert_eq!(page.links[0].url,"https://example.test/right/next");
 }
 #[test]fn metadata_preserves_internal_spaces_and_nonbreaking_spaces(){
  let page=extract("https://e.test/","<title>  שלום  |  עולם\u{a0} </title><meta name='description' content='  תיאור  עם רווחים\u{a0}  '>",0).unwrap();
  assert_eq!(page.title,"שלום  |  עולם\u{a0}");assert_eq!(page.description,"תיאור  עם רווחים\u{a0}");
 }
 #[test]fn readonly_missing_database(){let path=std::env::temp_dir().join(format!("fitig-absent-{}.sqlite",std::process::id()));assert!(!path.exists());assert!(load_pages(path.to_str().unwrap()).is_err());assert!(!path.exists())}
 #[cfg(feature="xlsx")]
 #[test]fn xlsx_preserves_text_and_numeric_types(){use std::io::Read;let folder=std::env::temp_dir().join(format!("fitig-xlsx-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.sqlite");{let c=open(db.to_str().unwrap()).unwrap();let p=Page{url:"https://example.test/%D7%A9%D7%9C%D7%95%D7%9D".into(),title:"=SUM(1,2) שלום".into(),status:200,depth:2,word_count:42,..Default::default()};c.execute("INSERT INTO pages VALUES(?1,?2)",params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}let out=folder.join("crawl.xlsx");export(db.to_str().unwrap(),out.to_str().unwrap(),"xlsx").unwrap();{let mut archive=zip::ZipArchive::new(std::fs::File::open(out).unwrap()).unwrap();let mut sheet=String::new();archive.by_name("xl/worksheets/sheet1.xml").unwrap().read_to_string(&mut sheet).unwrap();assert!(sheet.contains("<v>200</v>"));assert!(!sheet.contains("<f>"));let mut strings=String::new();archive.by_name("xl/sharedStrings.xml").unwrap().read_to_string(&mut strings).unwrap();assert!(strings.contains("=SUM(1,2) שלום"));assert!(strings.contains("https://example.test/שלום"));assert!(strings.contains("https://example.test/"));}std::fs::remove_dir_all(folder).unwrap();}
 #[test]fn compare_reports_changes(){let folder=std::env::temp_dir().join(format!("fitig-compare-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();let a=folder.join("a.sqlite");let b=folder.join("b.sqlite");for (path,title,status,extra) in [(&a,"Old",200,"removed"),(&b,"New",404,"added")]{let c=open(path.to_str().unwrap()).unwrap();for p in [Page{url:"https://example.test/".into(),title:title.into(),status,..Default::default()},Page{url:format!("https://example.test/{extra}"),..Default::default()}]{c.execute("INSERT INTO pages VALUES(?1,?2)",params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}}let result=compare(a.to_str().unwrap(),b.to_str().unwrap()).unwrap();assert_eq!(result["added"],serde_json::json!(["https://example.test/added"]));assert_eq!(result["removed"],serde_json::json!(["https://example.test/removed"]));assert_eq!(result["changed"][0]["fields"],serde_json::json!(["status","title"]));std::fs::remove_dir_all(folder).unwrap();}
 #[test]fn fixture_crawl_resume_analysis_exports(){
  use std::io::{Read,Write};use std::net::TcpListener;use std::sync::{Arc,atomic::{AtomicBool,Ordering}};
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();listener.set_nonblocking(true).unwrap();let origin=format!("http://{}",listener.local_addr().unwrap());let done=Arc::new(AtomicBool::new(false));let signal=done.clone();let external_port=listener.local_addr().unwrap().port();
  let worker=std::thread::spawn(move||{while !signal.load(Ordering::Relaxed){match listener.accept(){Ok((mut s,_))=>{s.set_nonblocking(false).unwrap();s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut buf=Vec::new();let mut byte=[0u8;1];while buf.len()<32768 && !buf.ends_with(b"\r\n\r\n"){match s.read(&mut byte){Ok(1)=>buf.push(byte[0]),_=>break}}let request=String::from_utf8_lossy(&buf);let path=request.split_whitespace().nth(1).unwrap_or("/");let (status,body)=match path{"/robots.txt"=>("200 OK","User-agent: *\nDisallow: /private"),"/"=>("200 OK","<title>Fixture</title><body><h1>Root</h1><a href='/missing'>bad</a><a href='/private'>blocked</a><a href='/next'>next</a><a href='/redirect-a'>redirect</a><img src='/image.png' alt='fixture'></body>"),"/redirect-a"=>("302 Found\r\nLocation: /redirect-b",""),"/redirect-b"=>("302 Found\r\nLocation: /redirect-a",""),"/image.png"=>("200 OK","fixture image"),"/external"=>("200 OK","<a href='/should-not-traverse'>external onward</a>"),"/next"=>("200 OK","<title>Next</title><body><h1>Next</h1></body>"),_=>("404 Not Found","<title>Error page</title><body><a href='/error-only'>Error navigation</a></body>")};let body=if path=="/"{format!("{body}<a href='http://localhost:{external_port}/external'>external</a>")}else{body.to_string()};let _=write!(s,"HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());},Err(_)=>std::thread::sleep(Duration::from_millis(5))}}});
  let folder=std::env::temp_dir().join(format!("fitig-test-{}-{}",std::process::id(),origin.rsplit(':').next().unwrap()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.db").to_string_lossy().into_owned();
  let first=crawl(CrawlOptions{url:format!("{origin}/"),db:db.clone(),limit:1,delay_ms:0,resume:false}, |_|{}).unwrap();assert_eq!(first.len(),1);assert_eq!(first[0].status,200,"Root fixture request failed: {}",first[0].error);assert_eq!(first[0].title,"Fixture");assert!(pending_count(&db).unwrap()>0,"Root did not enqueue discovered URLs");
  let all=crawl(CrawlOptions{url:format!("{origin}/"),db:db.clone(),limit:10,delay_ms:0,resume:true}, |_|{}).unwrap();assert_eq!(all.len(),7);assert!(!all.iter().any(|page|page.url.ends_with("/error-only")));assert!(all.iter().find(|page|page.url.ends_with("/missing")).unwrap().title.is_empty());assert_eq!(project_seed(&db).unwrap(),format!("{origin}/"));assert_eq!(pending_count(&db).unwrap(),0);assert_eq!(all.iter().filter(|p|p.status==302).count(),2);assert!(all.iter().any(|p|p.url.ends_with("/image.png")));assert!(all.iter().any(|p|p.error=="Blocked by robots.txt"));assert_eq!(analyze(&db).unwrap()["broken_links"].as_array().unwrap().len(),1);
  let csv=folder.join("report.csv");export(&db,csv.to_str().unwrap(),"csv").unwrap();assert!(std::fs::read_to_string(csv).unwrap().contains("Fixture"));let html=folder.join("report.html");export(&db,html.to_str().unwrap(),"html").unwrap();assert!(html.exists());assert_eq!(compare(&db,&db).unwrap()["changed"].as_array().unwrap().len(),0);
  let list_db=folder.join("list.sqlite").to_string_lossy().into_owned();let list=vec![format!("{origin}/"),format!("{}/next",origin.replace("127.0.0.1","localhost"))];let options=CrawlOptions{url:String::new(),db:list_db.clone(),limit:1,delay_ms:0,resume:false};assert_eq!(crawl_list(options.clone(),list.clone(),|_|{}).unwrap().len(),1);let mut options=options;options.resume=true;options.limit=0;let listed=crawl_list(options.clone(),list.clone(),|_|{}).unwrap();assert_eq!(listed.len(),2);assert!(listed.iter().all(|p|p.status==200));assert!(listed.iter().all(|p|p.depth==0));assert!(!listed.iter().any(|p|p.url.ends_with("/missing")));assert_eq!(project_mode(&list_db).unwrap(),"list");assert_eq!(project_urls(&list_db).unwrap(),list);assert_eq!(pending_count(&list_db).unwrap(),0);assert!(crawl_list(options.clone(),vec![format!("{origin}/")],|_|{}).is_err());let mut spider=options;spider.url=format!("{origin}/");assert!(crawl(spider,|_|{}).is_err());
  let rule=CustomRule{name:"Fixture title".into(),kind:"css".into(),expression:"title".into(),scope:"text".into(),..Default::default()};
  assert!(load_source(&db,&format!("{origin}/")).unwrap().unwrap().contains("<title>Fixture</title>"));
  assert_eq!(reprocess_custom(&db,vec![rule.clone()]).unwrap(),6);
  let reprocessed=load_pages(&db).unwrap();let root=reprocessed.iter().find(|p|p.url==format!("{origin}/")).unwrap();assert!(root.source_stored);assert_eq!(root.custom[0].values,vec!["Fixture"]);
  assert!(crawl(CrawlOptions{url:format!("{origin}/"),db:db.clone(),limit:0,delay_ms:0,resume:true},|_|{}).is_err());
  assert_eq!(crawl_configured(CrawlOptions{url:format!("{origin}/"),db:db.clone(),limit:0,delay_ms:0,resume:true},None,project_profile(&db).unwrap(),|_|{}).unwrap().len(),7);
  let custom_db=folder.join("custom.sqlite").to_string_lossy().into_owned();let profile=CrawlProfile{custom_rules:vec![rule],store_html:true,..Default::default()};let custom=crawl_configured(CrawlOptions{url:format!("{origin}/"),db:custom_db.clone(),limit:1,delay_ms:0,resume:false},None,profile.clone(),|_|{}).unwrap();assert_eq!(custom[0].custom[0].values,vec!["Fixture"]);assert_eq!(project_profile(&custom_db).unwrap(),profile);
  let external_db=folder.join("external.sqlite").to_string_lossy().into_owned();let external=crawl_configured(CrawlOptions{url:format!("{origin}/"),db:external_db,limit:0,delay_ms:0,resume:false},None,CrawlProfile{check_external:true,..Default::default()},|_|{}).unwrap();assert_eq!(external.len(),8);let checked=external.iter().find(|p|p.url.ends_with("/external")).unwrap();assert!(!checked.internal);assert_eq!(checked.status,200);assert!(checked.links.is_empty());assert!(!external.iter().any(|p|p.url.ends_with("/should-not-traverse")));
  let scoped_db=folder.join("scoped.sqlite").to_string_lossy().into_owned();let scoped=crawl_configured(CrawlOptions{url:format!("{origin}/"),db:scoped_db,limit:0,delay_ms:0,resume:false},None,CrawlProfile{max_depth:1,include_patterns:vec!["/next$".into()],user_agent:"FixtureBot/1.0".into(),..Default::default()},|_|{}).unwrap();assert_eq!(scoped.len(),2);assert!(scoped.iter().all(|p|p.depth<=1));assert!(scoped.iter().any(|p|p.url.ends_with("/next")));
  assert!(!robots_allowed_for("User-agent: FixtureBot\nDisallow: /private\nUser-agent: *\nAllow: /","/private","FixtureBot/1.0"));
  assert!(robots_allowed_for("User-agent: FixtureBot\nDisallow: /private\nUser-agent: *\nAllow: /","/private","AnotherBot/1.0"));
  // Browser navigation must change relative-link resolution, without changing
  // the stored HTTP request identity or widening the configured crawl scope.
  for (name,location,internal) in [("render-same",format!("{origin}/catalog/item#fragment"),true),("render-cross",format!("http://localhost:{external_port}/catalog/item"),false)] {
   let render_db=folder.join(format!("{name}.sqlite")).to_string_lossy().into_owned();
   let pages=crawl_configured_with_renderer(CrawlOptions{url:format!("{origin}/"),db:render_db.clone(),limit:1,delay_ms:0,resume:false},None,CrawlProfile{rendering:render::Rendering{enabled:true,..Default::default()},..Default::default()},|_,_|Ok(render::RenderedDocument{html:"<title>Rendered</title><body><a href='next'>next</a></body>".into(),url:location.clone(),..Default::default()}),|_|{}).unwrap();
   let page=&pages[0];assert_eq!(page.url,format!("{origin}/"));assert_eq!(page.status,200);assert!(page.rendered);assert_eq!(page.title,"Rendered");
   assert_eq!(page.rendered_url,location.split('#').next().unwrap());
   assert_eq!(page.links[0].url,Url::parse(&location).unwrap().join("next").unwrap().as_str());assert_eq!(page.links[0].internal,internal);
   assert_eq!(pending_count(&render_db).unwrap(),usize::from(internal));
   assert!(load_source(&render_db,&page.url).unwrap().unwrap().contains("Fixture"));
   assert!(load_rendered_source(&render_db,&page.url).unwrap().unwrap().contains("Rendered"));
  }
  done.store(true,Ordering::Relaxed);worker.join().unwrap();std::fs::remove_dir_all(folder).unwrap();
 }
}





#[cfg(test)]mod visualisation_tests{use super::*;#[test]fn body_words_capture_visible_unicode_and_load_legacy(){let p=extract("https://e.test/","<html><body>שלום שלום World world <script>secret secret</script><style>hidden hidden</style></body></html>",0).unwrap();let terms=p.body_terms.as_ref().unwrap();assert_eq!(terms.get("שלום"),Some(&2));assert_eq!(terms.get("world"),Some(&2));assert!(!terms.contains_key("secret"));assert!(!terms.contains_key("hidden"));let mut value=serde_json::to_value(&p).unwrap();value.as_object_mut().unwrap().remove("body_terms");assert!(serde_json::from_value::<Page>(value).unwrap().body_terms.is_none());}#[test]fn body_words_are_bounded_and_ranked(){let content=(0..600).map(|i|format!("word{i}")).collect::<Vec<_>>().join(" ")+" important important important";let terms=body_terms(&content);assert_eq!(terms.len(),512);assert_eq!(terms.get("important"),Some(&3));}}

#[cfg(test)]mod sitemap_unicode_tests{use super::*;#[test]fn sitemap_uses_readable_hebrew_and_preserves_url_separators(){let folder=std::env::temp_dir().join(format!("fitig-sitemap-unicode-{}",std::process::id()));std::fs::create_dir_all(&folder).unwrap();let db=folder.join("crawl.sqlite");{let c=open(db.to_str().unwrap()).unwrap();let p=Page{url:"https://e.test/%D7%A9%D7%9C%D7%95%D7%9D?a=%2F%26&b=%D7%90".into(),status:200,content_type:"text/html".into(),..Default::default()};c.execute("INSERT INTO pages VALUES(?1,?2)",params![p.url,serde_json::to_string(&p).unwrap()]).unwrap();}let out=folder.join("sitemap.xml");assert_eq!(export_sitemap(db.to_str().unwrap(),out.to_str().unwrap()).unwrap(),1);let xml=std::fs::read_to_string(&out).unwrap();assert!(xml.contains(r#"encoding="UTF-8""#));assert!(xml.contains("https://e.test/שלום?a=%2F%26&amp;b=א"));assert!(!xml.contains("%D7"));std::fs::remove_dir_all(folder).unwrap();}}

#[cfg(test)]mod content_area_tests{use super::*;#[test]fn excludes_nav_footer_from_counts_and_hash_but_preserves_body_cloud(){let a=extract("https://e.test/","<body><nav>menu navigation</nav><main>שלום עולם content</main><footer>footer address</footer><script>secret</script></body>",0).unwrap();let b=extract("https://e.test/","<body><nav>different menu</nav><main>שלום עולם content</main><footer>different footer</footer></body>",0).unwrap();assert_eq!(a.word_count,3);assert_eq!(a.content_hash,b.content_hash);assert!(a.body_terms.as_ref().unwrap().contains_key("navigation"));assert!(a.body_terms.as_ref().unwrap().contains_key("footer"));assert!(!a.body_terms.as_ref().unwrap().contains_key("secret"));}}

#[cfg(test)]mod project_start_safety_tests{
 use super::*;
 #[test]fn rejected_start_and_resume_leave_legacy_database_unmodified(){
  let folder=std::env::temp_dir().join(format!("fitig-start-safety-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&folder).unwrap();
  let path=folder.join("old.sqlite");
  {let c=rusqlite::Connection::open(&path).unwrap();c.execute_batch("CREATE TABLE pages(url TEXT PRIMARY KEY,json TEXT NOT NULL);CREATE TABLE frontier(url TEXT PRIMARY KEY,depth INTEGER NOT NULL,state TEXT NOT NULL DEFAULT 'pending');CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);INSERT INTO metadata VALUES('seed','https://e.test/');INSERT INTO metadata VALUES('mode','spider');INSERT INTO pages VALUES('https://e.test/','preserve');INSERT INTO frontier VALUES('https://e.test/',0,'pending');").unwrap();}
  let before=std::fs::read(&path).unwrap();
  for(resume,url,profile)in[(false,"https://e.test/",CrawlProfile::default()),(true,"https://different.test/",CrawlProfile::default()),(true,"https://e.test/",CrawlProfile{store_html:false,..Default::default()})]{
   let result=crawl_configured(CrawlOptions{url:url.into(),db:path.to_string_lossy().into_owned(),limit:0,delay_ms:0,resume},None,profile,|_|{});
   assert!(result.is_err());assert!(std::fs::read(&path).unwrap()==before,"Rejected request modified the database");assert_eq!(std::fs::read_dir(&folder).unwrap().count(),1);
  }
  std::fs::remove_dir_all(folder).unwrap();
 }
 #[test]fn resume_missing_project_never_creates_a_database(){
  let path=std::env::temp_dir().join(format!("fitig-missing-resume-{}-{}.sqlite",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  assert!(crawl(CrawlOptions{url:"https://e.test/".into(),db:path.to_string_lossy().into_owned(),limit:0,delay_ms:0,resume:true},|_|{}).is_err());assert!(!path.exists());
 }
}

#[cfg(test)]mod project_snapshot_tests{
 use super::*;
 #[test]fn project_data_reads_metadata_queue_and_pages_without_modifying_storage(){let folder=std::env::temp_dir().join(format!("fitig-project-view-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&folder).unwrap();let path=folder.join("project.sqlite");{let c=open(path.to_str().unwrap()).unwrap();c.execute_batch("INSERT INTO metadata VALUES('seed','https://e.test/');INSERT INTO metadata VALUES('mode','list');INSERT INTO metadata VALUES('profile','{\"store_html\":false}');INSERT INTO frontier VALUES('https://e.test/b',0,'pending');INSERT INTO frontier VALUES('https://e.test/a',0,'inflight');").unwrap();let page=Page{url:"https://e.test/a".into(),title:"שלום".into(),..Default::default()};c.execute("INSERT INTO pages VALUES(?1,?2)",params![page.url,serde_json::to_string(&page).unwrap()]).unwrap();}let before=std::fs::read(&path).unwrap();let view=load_project_data(path.to_str().unwrap()).unwrap();assert_eq!(view.pages.len(),1);assert_eq!(view.pages[0].title,"שלום");assert_eq!(view.list_urls,vec!["https://e.test/b","https://e.test/a"]);assert_eq!(view.queued,2);assert!(!view.profile.store_html);assert!(std::fs::read(&path).unwrap()==before);std::fs::remove_dir_all(folder).unwrap();}
 #[test]fn invalid_project_payload_and_missing_file_are_rejected_read_only(){let folder=std::env::temp_dir().join(format!("fitig-invalid-project-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&folder).unwrap();let path=folder.join("invalid.sqlite");assert!(load_project_data(path.to_str().unwrap()).is_err());assert!(!path.exists());{let c=open(path.to_str().unwrap()).unwrap();c.execute_batch("INSERT INTO metadata VALUES('seed','https://e.test/');INSERT INTO pages VALUES('https://e.test/','not json');").unwrap();}let before=std::fs::read(&path).unwrap();assert!(load_project_data(path.to_str().unwrap()).is_err());assert!(std::fs::read(&path).unwrap()==before);std::fs::remove_dir_all(folder).unwrap();}
}

#[cfg(test)]mod core_contract_tests{
 use super::*;
 #[test]fn h3_to_h6_keep_occurrences_unicode_and_entities(){let p=extract("https://e.test/","<h1>ראשי</h1><h3>שלוש <span>א</span></h3><h3>שלוש ב</h3><h4>ארבע</h4><h5>חמש &amp; עוד</h5><h6>שש</h6>",0).unwrap();assert_eq!(p.headings[2],vec!["שלוש א","שלוש ב"]);assert_eq!(p.headings[3],vec!["ארבע"]);assert_eq!(p.headings[4],vec!["חמש & עוד"]);assert_eq!(p.headings[5],vec!["שש"])}
 #[test]fn empty_h1_is_missing_but_a_nonempty_occurrence_is_not(){let empty=extract("https://e.test/","<h1> \n </h1>",0).unwrap();assert!(empty.issues.contains(&"Missing H1".into()));let mixed=extract("https://e.test/","<h1></h1><h1>תוכן</h1>",0).unwrap();assert!(!mixed.issues.contains(&"Missing H1".into()));assert!(mixed.issues.contains(&"Multiple H1".into()))}
}

#[cfg(test)]mod timeout_profile_tests{
 use super::*;
 #[test]fn legacy_defaults_and_bounds(){let old:CrawlProfile=serde_json::from_str("{}").unwrap();assert_eq!(old.request_timeout_secs,20);for seconds in [1,20,300]{assert!(validate_request_timeout(seconds).is_ok())}for seconds in [0,301,u64::MAX]{assert!(validate_request_timeout(seconds).is_err())}}
}

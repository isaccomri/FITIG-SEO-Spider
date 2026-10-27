//! Queue-only URL transformations. Original link/redirect observations remain evidence.
use crate::Result;
use serde::{Serialize,Deserialize};
use regex::{Regex,RegexBuilder};
use url::Url;
const URL_LIMIT:usize=16384;
#[derive(Debug,Clone,Default,Serialize,Deserialize,PartialEq,Eq)]#[serde(default)]
pub struct Rule{pub pattern:String,pub replacement:String}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]#[serde(default)]
pub struct Options{pub enabled:bool,pub remove_parameters:Vec<String>,pub remove_all_parameters:bool,pub parameter_case_sensitive:bool,pub rules:Vec<Rule>,pub lowercase:bool,pub percent_case:String}
impl Default for Options{fn default()->Self{Self{enabled:false,remove_parameters:vec![],remove_all_parameters:false,parameter_case_sensitive:true,rules:vec![],lowercase:false,percent_case:"preserve".into()}}}
enum Part{Literal(String),Group(usize)}
struct Replacement{regex:Regex,parts:Vec<Part>}
pub struct Program{options:Options,rules:Vec<Replacement>}
#[derive(Debug,Clone,Serialize,Deserialize)]pub struct Entry{pub original_url:String,pub crawl_url:Option<String>,pub disposition:String,pub error:String}
#[derive(Debug,Clone,Default,Serialize,Deserialize)]pub struct Report{pub changed:usize,pub errors:usize,pub omitted:usize,pub entries:Vec<Entry>,#[serde(skip)]bytes:usize}
impl Report{pub fn record(&mut self,entry:Entry){if !entry.error.is_empty(){self.errors+=1}else{self.changed+=1}let size=serde_json::to_vec(&entry).map_or(usize::MAX,|v|v.len());if self.entries.len()<2000&&self.bytes.saturating_add(size)<1024*1024{self.bytes+=size;self.entries.push(entry)}else{self.omitted+=1}}}
fn append(out:&mut String,value:&str)->Result<()>{if out.len().saturating_add(value.len())>URL_LIMIT{return Err("Rewritten URL exceeds 16 KiB".into())}out.push_str(value);Ok(())}
fn parts(regex:&Regex,replacement:&str)->Result<Vec<Part>>{
 let mut result=Vec::new();let mut literal=String::new();let mut chars=replacement.chars().peekable();
 while let Some(c)=chars.next(){if c!='$'{literal.push(c);continue}if chars.peek()==Some(&'$'){chars.next();literal.push('$');continue}
  let mut name=String::new();if chars.peek()==Some(&'{'){chars.next();let mut closed=false;for c in chars.by_ref(){if c=='}'{closed=true;break}name.push(c)}if !closed{return Err("Unclosed replacement capture; use ${name} or $1".into())}}else{while chars.peek().is_some_and(|c|c.is_ascii_alphanumeric()||*c=='_'){name.push(chars.next().unwrap())}}
  if name.is_empty(){literal.push('$');continue}
  let index=name.parse::<usize>().ok().filter(|index|*index<regex.captures_len()).or_else(||regex.capture_names().position(|candidate|candidate==Some(name.as_str()))).ok_or("Replacement references an undefined capture group")?;
  if !literal.is_empty(){result.push(Part::Literal(std::mem::take(&mut literal)))}result.push(Part::Group(index));
 }
 if !literal.is_empty(){result.push(Part::Literal(literal))}Ok(result)
}
impl Program{
 pub fn compile(options:&Options)->Result<Self>{
  if options.rules.len()>32||options.remove_parameters.len()>100||options.remove_parameters.iter().any(|p|p.is_empty()||p.len()>256||p.chars().any(char::is_control)){return Err("URL rewriting supports 32 rules and 100 nonempty parameter names up to 256 bytes".into())}
  if !["preserve","upper","lower"].contains(&options.percent_case.as_str()){return Err("Percent encoding case must be preserve, upper or lower".into())}
  let mut rules=Vec::new();let mut total=0;for rule in &options.rules{total+=rule.pattern.len()+rule.replacement.len();if rule.pattern.is_empty()||rule.pattern.len()>8192||rule.replacement.len()>8192||total>128*1024{return Err("URL rewrite expressions/replacements are limited to 8 KiB each and 128 KiB combined".into())}let regex=RegexBuilder::new(&rule.pattern).size_limit(1024*1024).build()?;rules.push(Replacement{parts:parts(&regex,&rule.replacement)?,regex})}
  Ok(Self{options:options.clone(),rules})
 }
 pub fn enabled(&self)->bool{self.options.enabled}
 pub fn apply(&self,raw:&str)->Result<String>{
  if !self.enabled(){return Ok(raw.into())}if raw.len()>URL_LIMIT{return Err("Discovered URL exceeds 16 KiB".into())}
  let mut parsed=Url::parse(raw)?;
  if self.options.remove_all_parameters{parsed.set_query(None)}else if !self.options.remove_parameters.is_empty(){if let Some(query)=parsed.query(){let retained=query.split('&').filter(|part|{let key=part.split('=').next().unwrap_or("");let decoded=url::form_urlencoded::parse(format!("{key}=").as_bytes()).next().map(|(k,_)|k.into_owned()).unwrap_or_default();!self.options.remove_parameters.iter().any(|name|if self.options.parameter_case_sensitive{decoded==*name}else{decoded.to_lowercase()==name.to_lowercase()})}).collect::<Vec<_>>();if retained.len()!=query.split('&').count(){let value=retained.join("&");parsed.set_query(if retained.is_empty(){None}else{Some(&value)})}}}
  let mut value=parsed.to_string();
  for rule in &self.rules{let mut out=String::new();let mut end=0;for captures in rule.regex.captures_iter(&value){let matched=captures.get(0).unwrap();append(&mut out,&value[end..matched.start()])?;for part in &rule.parts{match part{Part::Literal(value)=>append(&mut out,value)?,Part::Group(index)=>if let Some(value)=captures.get(*index){append(&mut out,value.as_str())?}}}end=matched.end()}append(&mut out,&value[end..])?;value=out}
  if self.options.lowercase{value=value.to_lowercase()}
  let mut parsed=Url::parse(&value).map_err(|_|"Rewrite did not produce an absolute URL")?;
  if !matches!(parsed.scheme(),"http"|"https")||parsed.host_str().is_none()||!parsed.username().is_empty()||parsed.password().is_some(){return Err("Rewrite must produce an absolute HTTP(S) URL without credentials".into())}parsed.set_fragment(None);value=parsed.to_string();
  if self.options.percent_case!="preserve"{let mut bytes=value.into_bytes();let mut i=0;while i+2<bytes.len(){if bytes[i]==b'%'&&bytes[i+1].is_ascii_hexdigit()&&bytes[i+2].is_ascii_hexdigit(){for j in [i+1,i+2]{bytes[j]=if self.options.percent_case=="upper"{bytes[j].to_ascii_uppercase()}else{bytes[j].to_ascii_lowercase()}}i+=3}else{i+=1}}value=String::from_utf8(bytes)?}if value.len()>URL_LIMIT{return Err("Rewritten URL exceeds 16 KiB".into())}Ok(value)
 }
}
pub fn preview(options:&Options,urls:&[String])->Result<Vec<Entry>>{if urls.len()>100{return Err("Preview accepts at most 100 URLs".into())}let program=Program::compile(options)?;Ok(urls.iter().map(|raw|match program.apply(raw){Ok(url)=>Entry{original_url:raw.clone(),disposition:if url==*raw{"Unchanged"}else{"Rewritten"}.into(),crawl_url:Some(url),error:String::new()},Err(error)=>Entry{original_url:raw.clone(),crawl_url:None,disposition:"Invalid".into(),error:error.to_string()}}).collect())}
pub fn save(connection:&rusqlite::Connection,url:&str,report:&Report)->Result<()>{let raw=serde_json::to_string(report)?;if raw.len()>2*1024*1024{return Err("Rewrite audit exceeds 2 MiB".into())}let key=format!("url-rewrites:{url}");connection.execute("DELETE FROM metadata WHERE key=?1",[&key])?;connection.execute("INSERT INTO metadata(key,value) VALUES(?1,?2)",rusqlite::params![key,raw])?;Ok(())}
pub fn load(db:&str,url:&str)->Result<Option<Report>>{use rusqlite::OptionalExtension;let connection=crate::open_read(db)?;let raw:Option<String>=connection.query_row("SELECT value FROM metadata WHERE key=?1",[format!("url-rewrites:{url}")],|row|row.get(0)).optional()?;raw.map(|raw|{if raw.len()>2*1024*1024{return Err("Recorded rewrite audit exceeds 2 MiB".into())}serde_json::from_str(&raw).map_err(Into::into)}).transpose()}

#[cfg(test)]mod tests{
 use super::*;
 fn active()->Options{Options{enabled:true,..Default::default()}}
 #[test]fn parameters_preserve_unselected_wire_values_and_unicode_names(){let p=Program::compile(&Options{remove_parameters:vec!["utm_source".into(),"שלום".into()],..active()}).unwrap();assert_eq!(p.apply("https://e.test/Path?a=%2f+%20&utm_source=x&%D7%A9%D7%9C%D7%95%D7%9D=1&a=2&UTM_SOURCE=y").unwrap(),"https://e.test/Path?a=%2f+%20&a=2&UTM_SOURCE=y");let insensitive=Program::compile(&Options{parameter_case_sensitive:false,remove_parameters:vec!["utm_source".into()],..active()}).unwrap();assert_eq!(insensitive.apply("https://e.test/?utm_source=1&UTM_SOURCE=2").unwrap(),"https://e.test/");}
 #[test]fn ordered_global_captures_literal_dollars_and_explicit_encoding_case(){let p=Program::compile(&Options{rules:vec![Rule{pattern:"/Old/(?P<id>[0-9]+)".into(),replacement:"/new/${id}".into()},Rule{pattern:"new".into(),replacement:"ITEM".into()}],lowercase:true,percent_case:"upper".into(),..active()}).unwrap();assert_eq!(p.apply("https://e.test/Old/12?x=%d7%a9#hash").unwrap(),"https://e.test/item/12?x=%D7%A9");let p=Program::compile(&Options{rules:vec![Rule{pattern:"[?]x".into(),replacement:"?$$x".into()}],..active()}).unwrap();assert_eq!(p.apply("https://e.test/?x").unwrap(),"https://e.test/?$x");}
 #[test]fn invalid_expressions_captures_schemes_credentials_and_expansion_are_bounded(){for rule in [Rule{pattern:"[".into(),replacement:"".into()},Rule{pattern:"x".into(),replacement:"$missing".into()},Rule{pattern:"(?=x)".into(),replacement:"".into()}]{assert!(Program::compile(&Options{rules:vec![rule],..active()}).is_err())}for replacement in ["javascript:alert(1)","https://user:secret@e.test/"]{let p=Program::compile(&Options{rules:vec![Rule{pattern:"^.*$".into(),replacement:replacement.into()}],..active()}).unwrap();assert!(p.apply("https://e.test/").is_err())}let p=Program::compile(&Options{rules:vec![Rule{pattern:"x".into(),replacement:"y".repeat(8192)}],..active()}).unwrap();assert!(p.apply("https://e.test/xxx").is_err());}
 #[test]fn disabled_options_do_not_transform_and_preview_is_side_effect_free(){let settings=Options{remove_all_parameters:true,..Default::default()};assert_eq!(preview(&settings,&["https://e.test/?x=1".into()]).unwrap()[0].disposition,"Unchanged");let p=Program::compile(&Options{remove_all_parameters:true,..active()}).unwrap();assert_eq!(p.apply("https://e.test/?x=1").unwrap(),"https://e.test/");assert!(preview(&settings,&vec!["https://e.test/".into();101]).is_err());}
 #[test]fn audit_keeps_full_totals_and_explicit_omissions(){let mut report=Report::default();for _ in 0..2005{report.record(Entry{original_url:"https://e.test/a".into(),crawl_url:Some("https://e.test/b".into()),error:String::new(),disposition:"Queued".into()})}assert_eq!((report.changed,report.entries.len(),report.omitted),(2005,2000,5));}
}

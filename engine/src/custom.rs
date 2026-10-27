//! Independent source-based search/extraction. Rules are validated before any crawl mutation.
use crate::{Html, Selector, Result};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize,Serialize};

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(default)]
pub struct CustomRule {
 pub name:String, pub kind:String, pub expression:String, pub scope:String,
 pub attribute:String, pub case_sensitive:bool, pub negate:bool,
}
impl Default for CustomRule{fn default()->Self{Self{name:String::new(),kind:"text".into(),expression:String::new(),scope:"html".into(),attribute:String::new(),case_sensitive:false,negate:false}}}
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct CustomResult {pub name:String,pub kind:String,pub count:usize,pub matched:bool,pub values:Vec<String>,pub truncated:bool,#[serde(default)]pub error:String}
enum Matcher{Pattern(Regex),Css(Selector),XPath(sxd_xpath::XPath)}
fn xpath_document(html:&Html)->sxd_document::Package{
 let package=sxd_document::Package::new();{
  let document=package.as_document();let source=html.root_element();let root=document.create_element(source.value().name());document.root().append_child(root);let mut stack=vec![(source,root)];
  while let Some((source,target))=stack.pop(){for(name,value)in source.value().attrs(){target.set_attribute_value(name,value);}for child in source.children(){if let Some(text)=child.value().as_text(){target.append_child(document.create_text(text));}else if let Some(element)=scraper::ElementRef::wrap(child){let node=document.create_element(element.value().name());target.append_child(node);stack.push((element,node));}}}
 }package
}
pub struct CustomProgram{rules:Vec<(CustomRule,Matcher)>}
impl CustomProgram {
 pub fn compile(rules:&[CustomRule])->Result<Self>{
  if rules.len()>100{return Err("At most 100 custom rules are supported".into())}
  let mut names=std::collections::HashSet::new();let mut compiled=Vec::new();
  for rule in rules{
   if rule.name.trim().is_empty()||!names.insert(rule.name.clone()){return Err("Custom rule names must be non-empty and unique".into())}
   if rule.name.len()>200||rule.expression.is_empty()||rule.expression.len()>8192{return Err(format!("{}: supply an expression of 1–8192 bytes and a name up to 200 bytes",rule.name).into())}
   if !["html","text","inner_html","attribute"].contains(&rule.scope.as_str()){return Err(format!("{}: unsupported scope",rule.name).into())}
   let matcher=match rule.kind.as_str(){
    "text"|"regex_search"|"regex_extract"=>{if !["html","text"].contains(&rule.scope.as_str()){return Err("Search/regex scope must be html or text".into())}let expression=if rule.kind=="text"{regex::escape(&rule.expression)}else{rule.expression.clone()};Matcher::Pattern(RegexBuilder::new(&expression).case_insensitive(!rule.case_sensitive).size_limit(2*1024*1024).build().map_err(|e|format!("{}: {e}",rule.name))?)},
    "css"=>{if !["text","inner_html","attribute"].contains(&rule.scope.as_str()){return Err("CSS extraction output must be text, inner_html or attribute".into())}if rule.scope=="attribute"&&rule.attribute.trim().is_empty(){return Err("CSS attribute extraction requires an attribute name".into())}Matcher::Css(Selector::parse(&rule.expression).map_err(|e|format!("{}: invalid CSS selector: {e}",rule.name))?)},
    "xpath"=>Matcher::XPath(sxd_xpath::Factory::new().build(&rule.expression).map_err(|e|format!("{}: invalid XPath: {e}",rule.name))?.ok_or("XPath expression is empty")?),
    _=>return Err(format!("{}: unsupported rule type",rule.name).into())};
   compiled.push((rule.clone(),matcher));
  }Ok(Self{rules:compiled})
 }
 pub fn run(&self,html:&str)->Vec<CustomResult>{
  let document=Html::parse_document(html);let text=document.select(&crate::sel("body")).next().map(crate::visible_text).unwrap_or_default();
  let xpath=if self.rules.iter().any(|(_,m)|matches!(m,Matcher::XPath(_))){Some(xpath_document(&document))}else{None};
  self.rules.iter().map(|(rule,matcher)|{
   let mut result=CustomResult{name:rule.name.clone(),kind:rule.kind.clone(),..Default::default()};
   let mut add=|value:String|{result.count+=1;if result.values.len()<1000{let end=value.char_indices().map(|(i,_)|i).take_while(|i|*i<=16000).last().unwrap_or(0);if value.len()>16000{result.values.push(value[..end].into());result.truncated=true}else{result.values.push(value)}}else{result.truncated=true}};
   match matcher{
    Matcher::Css(selector)=>for element in document.select(selector){match rule.scope.as_str(){"attribute"=>if let Some(value)=element.value().attr(rule.attribute.as_str()){add(value.into())},"inner_html"=>add(element.inner_html()),_=>add(crate::visible_text(element))}},
    Matcher::XPath(expression)=>{match expression.evaluate(&sxd_xpath::Context::new(),xpath.as_ref().unwrap().as_document().root()){Ok(sxd_xpath::Value::Nodeset(nodes))=>for node in nodes.document_order(){add(node.string_value())},Ok(value)=>add(value.string()),Err(error)=>result.error=error.to_string()}},
    Matcher::Pattern(pattern)=>{let input=if rule.scope=="text"{text.as_str()}else{html};if rule.kind=="regex_extract"{for captures in pattern.captures_iter(input){if captures.len()>1{for group in captures.iter().skip(1).flatten(){add(group.as_str().into())}}else if let Some(group)=captures.get(0){add(group.as_str().into())}}}else{for found in pattern.find_iter(input){add(found.as_str().into())}}}
   }
   result.matched=result.error.is_empty()&&if rule.negate{result.count==0}else{result.count>0};result
  }).collect()
 }
}

#[cfg(test)]mod tests{use super::*;
 fn rule(kind:&str,expression:&str,scope:&str)->CustomRule{CustomRule{name:kind.into(),kind:kind.into(),expression:expression.into(),scope:scope.into(),..Default::default()}}
 #[test]fn unicode_search_counts_visible_text_and_negation(){let mut r=rule("text","שלום","text");let program=CustomProgram::compile(&[r.clone()]).unwrap();assert_eq!(program.run("<script>שלום</script><p>שלום שלום</p>")[0].count,2);r.negate=true;assert!(CustomProgram::compile(&[r]).unwrap().run("<p>nothing</p>")[0].matched)}
 #[test]fn regex_captures_and_case_policy(){let mut r=rule("regex_extract",r#"price="(\d+)""#,"html");let values=CustomProgram::compile(&[r.clone()]).unwrap().run("<i PRICE=\"123\"></i>");assert_eq!(values[0].values,vec!["123"]);r.case_sensitive=true;assert_eq!(CustomProgram::compile(&[r]).unwrap().run("PRICE=\"123\"")[0].count,0)}
 #[test]fn css_text_attributes_and_inner_html(){let mut rules=vec![rule("css",".product","text"),rule("css","a[href]","attribute"),rule("css",".product","inner_html")];rules[0].name="Product".into();rules[1].name="Href".into();rules[1].attribute="href".into();rules[2].name="Markup".into();let results=CustomProgram::compile(&rules).unwrap().run("<div class=product>מוצר <b>חדש</b></div><a href='/עברית'>go</a>");assert_eq!(results[0].values,vec!["מוצר חדש"]);assert_eq!(results[1].values,vec!["/עברית"]);assert!(results[2].values[0].contains("<b>"))}
 #[test]fn invalid_rules_rejected_before_execution(){assert!(CustomProgram::compile(&[rule("regex_search","(","html")]).is_err());assert!(CustomProgram::compile(&[rule("css","[","text")]).is_err());let r=rule("text","a","html");assert!(CustomProgram::compile(&[r.clone(),r]).is_err())}
 #[test]fn extraction_is_bounded_and_reports_truncation(){let results=CustomProgram::compile(&[rule("css","i","text")]).unwrap().run(&"<i>x</i>".repeat(1002));assert_eq!(results[0].count,1002);assert_eq!(results[0].values.len(),1000);assert!(results[0].truncated)}
 #[test]fn xpath_extracts_nodes_attributes_and_scalar_values_from_html(){let expressions=["//h1","//a/@href","count(//a)","string(//h1)"];for expression in expressions{let result=CustomProgram::compile(&[rule("xpath",expression,"text")]).unwrap().run("<h1>שלום <b>עולם</b></h1><a href='/אחד'>one</a><a href='/שני'>two</a>");assert!(result[0].error.is_empty());if expression=="//a/@href"{assert_eq!(result[0].values,vec!["/אחד","/שני"])}else if expression.starts_with("count"){assert_eq!(result[0].values,vec!["2"])}else{assert_eq!(result[0].values,vec!["שלום עולם"])}}assert!(CustomProgram::compile(&[rule("xpath","//*[","text")]).is_err());}
}

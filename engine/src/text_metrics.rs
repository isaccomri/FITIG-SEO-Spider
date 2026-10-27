//! Independent source-text word counting, tested against explicit HTML fixtures.
//! This does not evaluate CSS visibility or claim a complete rendered-text model.
use scraper::ElementRef;
// Reference fixtures demonstrate an 80-UTF-16-unit formatting boundary between
// independently appended text runs. This is a source-text convention, not a
// linguistic word segmentation rule or a browser viewport measurement.
#[derive(Default)]struct FormattedText{value:String,column:usize}
impl FormattedText{
 fn boundary(&mut self){if !self.value.ends_with('\n'){self.value.push('\n')}self.column=0;}
 fn append(&mut self,raw:&str){
  let mut text=String::with_capacity(raw.len());let mut space=false;for character in raw.chars(){if character.is_ascii_whitespace()||character=='\u{a0}'{if !space{text.push(' ')}space=true}else{text.push(character);space=false}}
  if text.is_empty()||text==" "&&(self.value.ends_with(' ')||self.value.ends_with('\n')){return}
  let width=text.encode_utf16().count();if self.column+width<=80{self.value.push_str(&text);self.column+=width;return}
  let mut pieces:Vec<_>=text.split(' ').collect();while pieces.last()==Some(&""){pieces.pop();}let count=pieces.len();let ends_space=text.ends_with(' ');for(index,piece)in pieces.into_iter().enumerate(){let trailing=index+1<count||ends_space;let width=piece.encode_utf16().count()+usize::from(trailing);if self.column+width>80{self.boundary()}self.value.push_str(piece);if trailing{self.value.push(' ')}self.column+=width;}
 }
}
pub fn word_count(body:ElementRef<'_>)->usize{
 enum Event<'a>{Enter(ElementRef<'a>),Exit(ElementRef<'a>),Text(String)}
 fn block(name:&str)->bool{matches!(name,"address"|"article"|"aside"|"blockquote"|"body"|"caption"|"center"|"dd"|"details"|"dialog"|"div"|"dl"|"dt"|"fieldset"|"figcaption"|"figure"|"footer"|"form"|"h1"|"h2"|"h3"|"h4"|"h5"|"h6"|"header"|"hgroup"|"hr"|"li"|"main"|"nav"|"ol"|"p"|"pre"|"section"|"summary"|"table"|"tbody"|"td"|"tfoot"|"th"|"thead"|"tr"|"ul"|"br")}
 let mut stack=vec![Event::Enter(body)];let mut output=FormattedText::default();
 while let Some(event)=stack.pop(){match event{
  Event::Text(text)=>output.append(&text),
  Event::Enter(element)=>{
   let name=element.value().name();if matches!(name,"script"|"style"|"noscript"|"template"|"textarea"|"select"|"nav"|"footer"){continue}
   if block(name){output.boundary()}if name=="li"{output.append("* ")}
   stack.push(Event::Exit(element));
   for child in element.children().rev(){if let Some(text)=child.value().as_text(){stack.push(Event::Text(text.to_string()))}else if let Some(child)=ElementRef::wrap(child){stack.push(Event::Enter(child))}}
  },
  Event::Exit(element)=>{
   let name=element.value().name();if block(name){output.boundary()}
   // Preserve attached inline words, separating immediately adjacent runs of
   // independent labels/links. Formatting tags do not introduce word breaks.
   if matches!(name,"a"|"span"|"code"|"label"|"button")&&element.next_sibling().and_then(ElementRef::wrap).map(|next|next.value().name()==name).unwrap_or(false){output.append(" ")}
  }
 }}
 output.value.replace('|'," ").split_whitespace().count()
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn matches_recorded_reference_html_fixtures(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../tests/fixtures/reference-text-counts.json")).unwrap();
  for(name,case)in fixture["cases"].as_object().unwrap(){let document=scraper::Html::parse_document(case["html"].as_str().unwrap());let body=document.select(&scraper::Selector::parse("body").unwrap()).next().unwrap();assert_eq!(word_count(body),case["expected_words"].as_u64().unwrap() as usize,"Reference HTML fixture: {name}")}
 }
 #[test]fn handles_deep_markup_iteratively(){let html=format!("<body>{}שלום עולם{}</body>","<span>".repeat(2000),"</span>".repeat(2000));let document=scraper::Html::parse_document(&html);assert_eq!(word_count(document.select(&scraper::Selector::parse("body").unwrap()).next().unwrap()),2);}
}

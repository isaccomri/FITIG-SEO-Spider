//! Deterministic HTML decoding. This is not statistical language detection.
use encoding_rs::{Encoding,UTF_8,WINDOWS_1252};
use scraper::{Html,Selector};

pub struct DecodedHtml {pub text:String,pub charset:String,pub source:&'static str,pub had_errors:bool}

fn charset_parameter(value:&str)->Option<&str>{
 static PARAM:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
 let captures=PARAM.get_or_init(||regex::Regex::new(r#"(?i)(?:^|;)\s*charset\s*=\s*(?:"([^"]*)"|'([^']*)'|([^;\s]*))"#).unwrap()).captures(value)?;
 captures.get(1).or_else(||captures.get(2)).or_else(||captures.get(3)).map(|v|v.as_str().trim())
}

fn meta_encoding(bytes:&[u8])->Option<&'static Encoding>{
 let preview=String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
 let document=Html::parse_document(&preview);
 let selector=Selector::parse("meta").unwrap();
 for meta in document.select(&selector){
  let value=meta.value();
  let label=value.attr("charset").or_else(||{
   if value.attr("http-equiv").map(|v|v.eq_ignore_ascii_case("content-type")).unwrap_or(false){
    value.attr("content").and_then(charset_parameter)
   }else{None}
  });
  if let Some(encoding)=label.and_then(|label|Encoding::for_label(label.as_bytes())){
   // UTF-16 declarations inside an ASCII meta tag are treated as UTF-8.
   return Some(match encoding.name(){"UTF-16LE"|"UTF-16BE"=>UTF_8,"x-user-defined"=>WINDOWS_1252,_=>encoding})
  }
 }
 None
}

pub fn decode_html(bytes:&[u8],content_type:&str)->DecodedHtml{
 let (encoding,source)=if let Some((encoding,_))=Encoding::for_bom(bytes){(encoding,"BOM")}
 else if let Some(encoding)=charset_parameter(content_type).and_then(|label|Encoding::for_label(label.as_bytes())){(encoding,"HTTP header")}
 else if let Some(encoding)=meta_encoding(bytes){(encoding,"HTML meta")}
 else if std::str::from_utf8(bytes).is_ok(){(UTF_8,"UTF-8 fallback")}
 else{(WINDOWS_1252,"Windows-1252 fallback")};
 let (text,actual,had_errors)=encoding.decode(bytes);
 DecodedHtml{text:text.into_owned(),charset:actual.name().into(),source,had_errors}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn hebrew_windows1255_http_and_meta(){
  let html="<title>שלום עולם</title><body><a href='/עברית'>קישור</a></body>";
  let (bytes,_,errors)=encoding_rs::WINDOWS_1255.encode(html);assert!(!errors);
  let decoded=decode_html(&bytes,"text/html; CHARSET = 'windows-1255'");
  assert_eq!(decoded.text,html);assert_eq!(decoded.charset,"windows-1255");assert!(!decoded.had_errors);
  let html=format!("<meta charset=windows-1255>{html}");let (bytes,_,_)=encoding_rs::WINDOWS_1255.encode(&html);
  let decoded=decode_html(&bytes,"text/html");assert_eq!(decoded.text,html);assert_eq!(decoded.source,"HTML meta");
 }
 #[test]fn bom_overrides_http_and_http_overrides_meta(){
  let bytes=b"\xef\xbb\xbf<meta charset=windows-1255><title>\xd7\xa9\xd7\x9c\xd7\x95\xd7\x9d</title>";
  let decoded=decode_html(bytes,"text/html; charset=windows-1252");assert_eq!(decoded.charset,"UTF-8");assert!(decoded.text.contains("שלום"));assert!(!decoded.text.starts_with('\u{feff}'));
  let decoded=decode_html(b"<meta charset=windows-1255>\xe9","text/html; charset=windows-1252");assert!(decoded.text.ends_with('é'));assert_eq!(decoded.source,"HTTP header");
 }
 #[test]fn ignores_fake_meta_and_bounds_prescan(){
  let decoded=decode_html(b"<!-- <meta charset=windows-1255> --><script>const x='<meta charset=windows-1255>';</script>\xe9","text/html");
  assert_eq!(decoded.source,"Windows-1252 fallback");
  let html=" ".repeat(1024)+"<meta charset=windows-1255>\u{e9}";
  let decoded=decode_html(html.as_bytes(),"text/html");assert_eq!(decoded.source,"UTF-8 fallback");
 }
 #[test]fn legacy_pragma_invalid_labels_and_decode_errors(){
  let bytes=b"<meta http-equiv='Content-Type' content='text/html; charset=windows-1255'>\xf9\xec\xe5\xed";
  assert!(decode_html(bytes,"text/html; charset=unknown").text.ends_with("שלום"));
  let decoded=decode_html(b"<meta charset=utf-8>\xff","text/html");assert!(decoded.had_errors);assert!(decoded.text.contains('\u{fffd}'));
  assert_eq!(decode_html(b"<meta charset=utf-16>ok","text/html").charset,"UTF-8");
 }
 #[test]fn utf16_bom_and_utf8_hebrew_without_declaration(){
  let text="<title>עברית</title>";let mut bytes=vec![0xff,0xfe];for unit in text.encode_utf16(){bytes.extend_from_slice(&unit.to_le_bytes())}
  let decoded=decode_html(&bytes,"text/html");assert_eq!(decoded.text,text);assert_eq!(decoded.charset,"UTF-16LE");assert!(!decoded.had_errors);
  assert_eq!(decode_html(text.as_bytes(),"text/html").text,text);
 }
}

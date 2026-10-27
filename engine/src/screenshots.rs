//! Bounded PNG artifacts kept outside the workbench's page JSON payload.
use crate::Result;
use base64::{Engine,engine::general_purpose::STANDARD};
use rusqlite::OptionalExtension;
pub const MAX_PNG:usize=8*1024*1024;
pub fn dimensions(bytes:&[u8])->Result<(u32,u32)>{
 if bytes.len()>MAX_PNG||bytes.len()<24||&bytes[..8]!=b"\x89PNG\r\n\x1a\n"||&bytes[12..16]!=b"IHDR"{return Err("Screenshot is missing a bounded PNG header".into())}
 let width=u32::from_be_bytes(bytes[16..20].try_into()?);let height=u32::from_be_bytes(bytes[20..24].try_into()?);if width==0||height==0||width>8192||height>8192{return Err("Screenshot dimensions are outside 1–8192 pixels".into())}Ok((width,height))
}
pub fn decode(encoded:&str)->Result<Vec<u8>>{if encoded.len()>(MAX_PNG+2)/3*4{return Err("Screenshot exceeds 8 MiB".into())}let bytes=STANDARD.decode(encoded)?;dimensions(&bytes)?;Ok(bytes)}
pub fn data_url(bytes:&[u8])->Result<String>{dimensions(bytes)?;Ok(format!("data:image/png;base64,{}",STANDARD.encode(bytes)))}
pub fn load(db:&str,url:&str)->Result<Option<Vec<u8>>>{let c=crate::open_read(db)?;let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='screenshots')",[],|r|r.get(0))?;if !exists{return Ok(None)}let bytes:Option<Vec<u8>>=c.query_row("SELECT png FROM screenshots WHERE url=?1",[url],|r|r.get(0)).optional()?;if let Some(bytes)=&bytes{dimensions(bytes)?;}Ok(bytes)}
pub fn export(db:&str,url:&str,path:&str)->Result<()>{crate::export_target::check(path)?;if !std::path::Path::new(path).extension().and_then(|value|value.to_str()).map(|value|value.eq_ignore_ascii_case("png")).unwrap_or(false){return Err("Choose a .png file".into())}let bytes=load(db,url)?.ok_or("No saved screenshot for this URL")?;std::fs::write(path,bytes)?;Ok(())}
#[cfg(test)]mod tests{
 use super::*;
 fn header(width:u32,height:u32)->Vec<u8>{let mut bytes=b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();bytes.extend(width.to_be_bytes());bytes.extend(height.to_be_bytes());bytes}
 #[test]fn bounded_header_and_base64_validation(){let bytes=header(1365,768);assert_eq!(dimensions(&bytes).unwrap(),(1365,768));assert_eq!(decode(&STANDARD.encode(&bytes)).unwrap(),bytes);for bytes in [vec![],b"not an image".to_vec(),header(0,1),header(8193,1)]{assert!(dimensions(&bytes).is_err())}assert!(decode("%%%invalid").is_err());assert!(data_url(&header(1,1)).unwrap().starts_with("data:image/png;base64,"));}
 #[test]fn stored_artifact_export_preserves_bytes_and_rejects_missing_and_wrong_extension(){let dir=std::env::temp_dir().join(format!("fitig-png-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir).unwrap();let db=dir.join("crawl.sqlite");let png=header(640,480);{let c=crate::open(db.to_str().unwrap()).unwrap();c.execute("INSERT INTO metadata VALUES('seed','https://e.test/')",[]).unwrap();c.execute("INSERT INTO screenshots VALUES('https://e.test/',?1)",[&png]).unwrap();}let db=db.to_str().unwrap();let before=std::fs::read(db).unwrap();let out=dir.join("שלום.png");export(db,"https://e.test/",out.to_str().unwrap()).unwrap();assert_eq!(std::fs::read(out).unwrap(),png);assert!(export(db,"https://e.test/",dir.join("bad.txt").to_str().unwrap()).is_err());assert!(load(db,"https://e.test/missing").unwrap().is_none());assert!(std::fs::read(db).unwrap()==before);std::fs::remove_dir_all(dir).unwrap();}
}

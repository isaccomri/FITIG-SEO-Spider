//! Bounded validation of opaque RGB/RGBA PNG chart exports before opening their destination.
use crate::Result;
use std::io::Read;
fn crc(bytes:&[u8])->u32{let mut crc=!0u32;for byte in bytes{crc^=u32::from(*byte);for _ in 0..8{crc=(crc>>1)^(0xedb88320u32.wrapping_mul(crc&1));}}!crc}
pub fn png(content:&str)->Result<Vec<u8>>{
 let encoded=content.strip_prefix("data:image/png;base64,").ok_or("Expected a PNG data URL")?;
 let bytes=crate::screenshots::decode(encoded)?;let (width,height)=crate::screenshots::dimensions(&bytes)?;
 if width>4096||height>4096{return Err("Chart PNG dimensions exceed 4096 pixels".into())}
 let mut offset=8;let mut first=true;let mut ended=false;let mut compressed=Vec::new();let mut channels=0usize;
 while offset<bytes.len(){
  if bytes.len()-offset<12{return Err("Truncated PNG chunk".into())}
  let length=u32::from_be_bytes(bytes[offset..offset+4].try_into()?) as usize;let end=offset.checked_add(12).and_then(|n|n.checked_add(length)).filter(|n|*n<=bytes.len()).ok_or("Invalid PNG chunk length")?;
  let kind=&bytes[offset+4..offset+8];let data=&bytes[offset+8..end-4];let stored=u32::from_be_bytes(bytes[end-4..end].try_into()?);
  if crc(&bytes[offset+4..end-4])!=stored{return Err("Invalid PNG chunk checksum".into())}
  if first&&kind!=b"IHDR"{return Err("PNG must begin with IHDR".into())}
  match kind{
   b"IHDR"=>{if !first||length!=13||data[8]!=8||!matches!(data[9],2|6)||data[10..13]!=[0,0,0]{return Err("Chart PNG must be non-interlaced 8-bit RGB/RGBA".into())}channels=if data[9]==6{4}else{3};},
   b"IDAT"=>compressed.extend_from_slice(data),
   b"IEND"=>{if length!=0||end!=bytes.len(){return Err("Invalid PNG ending".into())}ended=true;},
   _=>{if kind[0]&0x20==0{return Err("Unsupported critical PNG chunk".into())}},
  }
  first=false;offset=end;
 }
 if !ended||compressed.is_empty()||channels==0{return Err("Incomplete PNG image".into())}
 let stride=width as usize*channels+1;let expected=stride*height as usize;let mut decoded=Vec::new();flate2::read::ZlibDecoder::new(compressed.as_slice()).take(expected as u64+1).read_to_end(&mut decoded).map_err(|_|"Invalid PNG compressed pixels")?;
 if decoded.len()!=expected||decoded.chunks_exact(stride).any(|row|row[0]>4){return Err("Invalid PNG pixel rows".into())}Ok(bytes)
}
pub fn export(path:&str,content:&str)->Result<()>{
 if !std::path::Path::new(path).extension().and_then(|value|value.to_str()).map(|value|value.eq_ignore_ascii_case("png")).unwrap_or(false){return Err("Choose a .png file".into())}
 let bytes=png(content)?;
 if let Ok(mut file)=std::fs::File::open(path){let mut header=[0;16];if file.read_exact(&mut header).is_ok()&&&header==b"SQLite format 3\0"{return Err("Cannot overwrite a SQLite project with a chart".into())}}
 std::fs::write(path,bytes)?;Ok(())
}
#[cfg(test)]mod tests{
 use super::*;use std::io::Write;use base64::{Engine,engine::general_purpose::STANDARD};
 fn chunk(output:&mut Vec<u8>,kind:&[u8;4],data:&[u8]){output.extend((data.len() as u32).to_be_bytes());let start=output.len();output.extend(kind);output.extend(data);let checksum=crc(&output[start..]);output.extend(checksum.to_be_bytes());}
 fn image(width:u32,filter:u8)->String{let mut bytes=b"\x89PNG\r\n\x1a\n".to_vec();let mut header=width.to_be_bytes().to_vec();header.extend(1u32.to_be_bytes());header.extend([8,6,0,0,0]);chunk(&mut bytes,b"IHDR",&header);let mut compressor=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::default());compressor.write_all(&[filter,255,0,0,255]).unwrap();chunk(&mut bytes,b"IDAT",&compressor.finish().unwrap());chunk(&mut bytes,b"IEND",&[]);format!("data:image/png;base64,{}",STANDARD.encode(bytes))}
 #[test]fn valid_png_and_checksum_filter_bounds(){let valid=image(1,0);assert!(png(&valid).is_ok());assert!(png(&image(4097,0)).is_err());assert!(png(&image(2,0)).is_err());assert!(png(&image(1,5)).is_err());let mut bytes=STANDARD.decode(valid.strip_prefix("data:image/png;base64,").unwrap()).unwrap();bytes[30]^=1;assert!(png(&format!("data:image/png;base64,{}",STANDARD.encode(bytes))).is_err());assert!(png("data:image/jpeg;base64,invalid").is_err());}
 #[test]fn invalid_export_and_sqlite_collision_preserve_existing_bytes(){let folder=std::env::temp_dir().join(format!("fitig-chart-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&folder).unwrap();let destination=folder.join("תרשים.png");std::fs::write(&destination,b"SQLite format 3\0owned fixture").unwrap();assert!(export(destination.to_str().unwrap(),&image(1,0)).is_err());assert_eq!(std::fs::read(&destination).unwrap(),b"SQLite format 3\0owned fixture");let valid=folder.join("valid.png");export(valid.to_str().unwrap(),&image(1,0)).unwrap();let before=std::fs::read(&valid).unwrap();assert!(export(valid.to_str().unwrap(),"invalid").is_err());assert_eq!(std::fs::read(valid).unwrap(),before);assert!(export(folder.join("wrong.txt").to_str().unwrap(),&image(1,0)).is_err());std::fs::remove_dir_all(folder).unwrap();}
}

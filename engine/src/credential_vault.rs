//! Explicit, per-service Windows Credential Manager storage. No secrets in reports.
pub const MAX_BYTES:usize=2560;
pub fn validate_service(service:&str)->Result<(),String>{if matches!(service,"gsc"|"ga4"|"psi"|"inspection"){Ok(())}else{Err("Unsupported credential service".into())}}
pub fn validate_secret(secret:&str)->Result<(),String>{if secret.is_empty()||secret.len()>MAX_BYTES||!secret.bytes().all(|b|b.is_ascii_graphic()){Err("Credential must contain 1–2560 ASCII characters without whitespace".into())}else{Ok(())}}
pub struct Store{prefix:String}
impl Default for Store{fn default()->Self{Self{prefix:"FITIG SEO Spider/API/v1/".into()}}}
impl Store{
 fn target(&self,service:&str)->Result<String,String>{validate_service(service)?;Ok(format!("{}{service}",self.prefix))}
 pub fn exists(&self,service:&str)->Result<bool,String>{self.read(service).map(|v|v.is_some())}
 pub fn read(&self,service:&str)->Result<Option<String>,String>{let target=self.target(service)?;backend::read(&target)}
 pub fn save(&self,service:&str,secret:&str)->Result<(),String>{let target=self.target(service)?;validate_secret(secret)?;backend::save(&target,secret)}
 pub fn remove(&self,service:&str)->Result<(),String>{let target=self.target(service)?;backend::remove(&target)}
}
#[cfg(windows)]mod backend{
 use std::{ptr,ffi::c_void};
 #[repr(C)]struct FileTime{low:u32,high:u32}
 #[repr(C)]struct Credential{flags:u32,kind:u32,target:*mut u16,comment:*mut u16,written:FileTime,size:u32,blob:*mut u8,persist:u32,attribute_count:u32,attributes:*mut c_void,alias:*mut u16,user:*mut u16}
 #[link(name="Advapi32")]extern "system"{fn CredWriteW(credential:*const Credential,flags:u32)->i32;fn CredReadW(target:*const u16,kind:u32,flags:u32,credential:*mut *mut Credential)->i32;fn CredDeleteW(target:*const u16,kind:u32,flags:u32)->i32;fn CredFree(buffer:*mut c_void);}
 #[link(name="Kernel32")]extern "system"{fn GetLastError()->u32;}
 fn wide(value:&str)->Vec<u16>{value.encode_utf16().chain(Some(0)).collect()}
 fn error(operation:&str)->String{format!("Windows credential {operation} failed (code {}); secret omitted",unsafe{GetLastError()})}
 pub fn read(target:&str)->Result<Option<String>,String>{let name=wide(target);let mut raw=ptr::null_mut();if unsafe{CredReadW(name.as_ptr(),1,0,&mut raw)}==0{return if unsafe{GetLastError()}==1168{Ok(None)}else{Err(error("read"))}}if raw.is_null(){return Err("Windows returned no credential buffer".into())}
  struct Guard(*mut Credential);impl Drop for Guard{fn drop(&mut self){unsafe{CredFree(self.0.cast())}}}let guard=Guard(raw);let record=unsafe{&*guard.0};if record.size as usize>super::MAX_BYTES||record.size==0||record.blob.is_null(){return Err("Saved credential has invalid size; replace or remove it explicitly".into())}let bytes=unsafe{std::slice::from_raw_parts(record.blob,record.size as usize)};let secret=std::str::from_utf8(bytes).map_err(|_|"Saved credential has invalid encoding")?;super::validate_secret(secret)?;Ok(Some(secret.to_owned()))
 }
 pub fn save(target:&str,secret:&str)->Result<(),String>{let mut name=wide(target);let mut user=wide("FITIG SEO Spider");let record=Credential{flags:0,kind:1,target:name.as_mut_ptr(),comment:ptr::null_mut(),written:FileTime{low:0,high:0},size:secret.len()as u32,blob:secret.as_ptr()as *mut u8,persist:2,attribute_count:0,attributes:ptr::null_mut(),alias:ptr::null_mut(),user:user.as_mut_ptr()};if unsafe{CredWriteW(&record,0)}==0{Err(error("save"))}else{Ok(())}}
 pub fn remove(target:&str)->Result<(),String>{let name=wide(target);if unsafe{CredDeleteW(name.as_ptr(),1,0)}==0&&unsafe{GetLastError()}!=1168{Err(error("remove"))}else{Ok(())}}
}
#[cfg(not(windows))]mod backend{pub fn read(_: &str)->Result<Option<String>,String>{Err("Credential storage requires Windows Credential Manager".into())}pub fn save(_: &str,_:&str)->Result<(),String>{Err("Credential storage requires Windows Credential Manager".into())}pub fn remove(_: &str)->Result<(),String>{Err("Credential storage requires Windows Credential Manager".into())}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn bounds_and_service_allowlist(){for service in ["gsc","ga4","psi","inspection"]{assert!(validate_service(service).is_ok())}for value in ["","../gsc","GSC","gsc\0"]{assert!(validate_service(value).is_err())}for secret in ["","secret\nline","bad secret","עברית"]{assert!(validate_secret(secret).is_err())}assert!(validate_secret(&"a".repeat(2560)).is_ok());assert!(validate_secret(&"a".repeat(2561)).is_err())}
 #[cfg(windows)]#[test]fn native_store_roundtrip_replace_isolation_and_remove(){
  let store=Store{prefix:format!("FITIG SEO Spider/tests/{}/{}/",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())};
  struct Cleanup<'a>(&'a Store);impl Drop for Cleanup<'_>{fn drop(&mut self){let _=self.0.remove("gsc");let _=self.0.remove("psi");}}let _cleanup=Cleanup(&store);
  assert!(!store.exists("gsc").unwrap());store.save("gsc","owned-dummy-first").unwrap();assert_eq!(store.read("gsc").unwrap().as_deref(),Some("owned-dummy-first"));assert!(store.read("psi").unwrap().is_none());assert!(store.save("gsc","invalid whitespace").is_err());assert_eq!(store.read("gsc").unwrap().unwrap(),"owned-dummy-first");store.save("gsc","owned-dummy-second").unwrap();assert_eq!(store.read("gsc").unwrap().unwrap(),"owned-dummy-second");store.remove("gsc").unwrap();store.remove("gsc").unwrap();assert!(store.read("gsc").unwrap().is_none())
 }
}

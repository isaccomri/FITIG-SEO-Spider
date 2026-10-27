//! Versioned project ownership and additive, backed-up legacy migration.
use crate::Result;
use rusqlite::{Connection,OpenFlags};
use std::path::Path;
const APPLICATION_ID:i32=0x46495449;
const VERSION:i32=4;
const SCHEMA:&str="CREATE TABLE IF NOT EXISTS pages(url TEXT PRIMARY KEY,json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS frontier(url TEXT PRIMARY KEY,depth INTEGER NOT NULL,state TEXT NOT NULL DEFAULT 'pending'); CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE IF NOT EXISTS documents(url TEXT PRIMARY KEY,html TEXT NOT NULL); CREATE TABLE IF NOT EXISTS rendered_documents(url TEXT PRIMARY KEY,html TEXT NOT NULL); CREATE TABLE IF NOT EXISTS screenshots(url TEXT PRIMARY KEY,png BLOB NOT NULL); CREATE TABLE IF NOT EXISTS browser_logs(url TEXT PRIMARY KEY,json TEXT NOT NULL);";
fn inspect(connection:&Connection,allow_empty:bool)->Result<(i32,i32)>{
 let application:i32=connection.query_row("PRAGMA application_id",[],|row|row.get(0))?;
 let version:i32=connection.query_row("PRAGMA user_version",[],|row|row.get(0))?;
 if version>VERSION{return Err(format!("Project schema {version} is newer than supported schema {VERSION}; file was not modified").into())}
 if application!=0&&application!=APPLICATION_ID{return Err("This SQLite database belongs to another application; file was not modified".into())}
 if application==0{
  let tables:usize=connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|row|row.get(0))?;
  if allow_empty&&tables==0{return Ok((application,version))}
  let required:usize=connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('pages','frontier','metadata')",[],|row|row.get(0))?;
  if required!=3{return Err("Not a recognized FITIG project; file was not modified".into())}
  let seeds:usize=connection.query_row("SELECT COUNT(*) FROM metadata WHERE key='seed' AND length(value)>0",[],|row|row.get(0))?;
  if seeds!=1{return Err("Legacy FITIG project is missing its crawl seed; file was not modified".into())}
 }
 Ok((application,version))
}
pub(crate) fn open(path:&str)->Result<Connection>{
 let file=Path::new(path);let existed=file.exists();
 if let Some(parent)=file.parent(){if !parent.as_os_str().is_empty(){std::fs::create_dir_all(parent)?}}
 let mut connection=Connection::open(file)?;
 let(application,version)=inspect(&connection,!existed)?;
 if existed&&(application!=APPLICATION_ID||version<VERSION){
  let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
  let backup=file.with_file_name(format!("{}.schema-v{version}-backup-{nonce}.sqlite",file.file_name().ok_or("Project needs a filename")?.to_string_lossy()));
  connection.backup(rusqlite::DatabaseName::Main,&backup,None)?;
  // Verify the backup is a readable SQLite snapshot before any migration write.
  let snapshot=Connection::open_with_flags(&backup,OpenFlags::SQLITE_OPEN_READ_ONLY)?;
  let integrity:String=snapshot.query_row("PRAGMA quick_check",[],|row|row.get(0))?;
  if integrity!="ok"{return Err("Legacy backup failed integrity verification; original project not migrated".into())}
 }
 connection.execute_batch("PRAGMA journal_mode=WAL;")?;
 let transaction=connection.transaction()?;transaction.execute_batch(SCHEMA)?;
 transaction.pragma_update(None,"application_id",APPLICATION_ID)?;
 transaction.pragma_update(None,"user_version",VERSION)?;transaction.commit()?;
 Ok(connection)
}
pub(crate) fn read(path:&str)->Result<Connection>{let connection=Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_ONLY)?;inspect(&connection,false)?;Ok(connection)}
#[cfg(test)]mod tests{
 use super::*;
 fn folder(name:&str)->std::path::PathBuf{let directory=std::env::temp_dir().join(format!("fitig-db-{name}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&directory).unwrap();directory}
 #[test]fn legacy_migration_keeps_rows_and_creates_verified_snapshot(){
  let directory=folder("legacy");let path=directory.join("legacy.sqlite");
  {let connection=Connection::open(&path).unwrap();connection.execute_batch("CREATE TABLE pages(url TEXT PRIMARY KEY,json TEXT NOT NULL);CREATE TABLE frontier(url TEXT PRIMARY KEY,depth INTEGER NOT NULL,state TEXT NOT NULL DEFAULT 'pending');CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);INSERT INTO metadata VALUES('seed','https://e.test/');INSERT INTO pages VALUES('https://e.test/','original JSON');INSERT INTO frontier VALUES('https://e.test/',0,'done');").unwrap();}
  {let connection=open(path.to_str().unwrap()).unwrap();assert_eq!(connection.query_row("SELECT json FROM pages",[],|row|row.get::<_,String>(0)).unwrap(),"original JSON");assert_eq!(connection.query_row("PRAGMA user_version",[],|row|row.get::<_,i32>(0)).unwrap(),VERSION);}
  let backups:Vec<_>=std::fs::read_dir(&directory).unwrap().filter_map(|entry|{let path=entry.unwrap().path();path.file_name().unwrap().to_string_lossy().contains("backup").then_some(path)}).collect();assert_eq!(backups.len(),1);
  {let backup=read(backups[0].to_str().unwrap()).unwrap();assert_eq!(backup.query_row("PRAGMA user_version",[],|row|row.get::<_,i32>(0)).unwrap(),0);assert_eq!(backup.query_row("SELECT json FROM pages",[],|row|row.get::<_,String>(0)).unwrap(),"original JSON");}
  drop(open(path.to_str().unwrap()).unwrap());assert_eq!(std::fs::read_dir(&directory).unwrap().filter(|entry|entry.as_ref().unwrap().file_name().to_string_lossy().contains("backup")).count(),1);std::fs::remove_dir_all(directory).unwrap();
 }
 #[test]fn rejects_foreign_and_future_projects_without_modifying_bytes(){
  for(kind,sql)in[("foreign","PRAGMA application_id=123;CREATE TABLE unrelated(secret TEXT);INSERT INTO unrelated VALUES('preserve');"),("future","PRAGMA user_version=99;CREATE TABLE pages(url TEXT);")]{let directory=folder(kind);let path=directory.join("project.sqlite");{let connection=Connection::open(&path).unwrap();connection.execute_batch(sql).unwrap();}let before=std::fs::read(&path).unwrap();assert!(open(path.to_str().unwrap()).is_err());assert!(read(path.to_str().unwrap()).is_err());assert_eq!(std::fs::read(&path).unwrap(),before);std::fs::remove_dir_all(directory).unwrap();}
 }
 #[test]fn new_project_has_identity_and_read_does_not_create_missing_files(){let directory=folder("new");let path=directory.join("new.sqlite");assert!(read(path.to_str().unwrap()).is_err());assert!(!path.exists());{let connection=open(path.to_str().unwrap()).unwrap();assert_eq!(connection.query_row("PRAGMA application_id",[],|row|row.get::<_,i32>(0)).unwrap(),APPLICATION_ID);}assert!(read(path.to_str().unwrap()).is_ok());std::fs::remove_dir_all(directory).unwrap();}
}

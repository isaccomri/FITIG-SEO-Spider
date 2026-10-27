//! Refuse to truncate an existing SQLite database, regardless of its filename extension.
pub fn check(path:&str)->crate::Result<()> {
    use std::io::Read;
    match std::fs::File::open(path) {
        Ok(mut file)=>{
            let mut bytes=[0;16];let mut read=0;
            while read<bytes.len(){let count=file.read(&mut bytes[read..])?;if count==0{break}read+=count;}
            if read==16 && &bytes==b"SQLite format 3\0"{return Err("Export cannot overwrite a SQLite database; choose a report file".into())}
        },
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},
        Err(error)=>return Err(error.into()),
    }
    Ok(())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn specialist_exports_preserve_disguised_databases(){
        let root=std::env::temp_dir().join(format!("fitig-specialist-guard-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&root).unwrap();
        for extension in ["png","zip"]{
            let file=root.join(format!("database.{extension}"));std::fs::write(&file,b"SQLite format 3\0owned fixture").unwrap();let original=std::fs::read(&file).unwrap();let path=file.to_str().unwrap();
            let error=if extension=="png"{crate::screenshots::export("not-opened.sqlite","https://e.test/",path).unwrap_err()}else{crate::sitemap_export::export_pages(&[],"https://e.test/",path,Default::default()).unwrap_err()};
            assert!(error.to_string().contains("SQLite"));assert_eq!(std::fs::read(&file).unwrap(),original);
        }
        assert_eq!(std::fs::read_dir(&root).unwrap().count(),2);std::fs::remove_dir_all(root).unwrap();
    }
    #[test] fn report_exports_preserve_databases_even_with_report_extensions(){
        let root=std::env::temp_dir().join(format!("fitig-export-guard-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&root).unwrap();
        let out=root.join("disguised.csv");std::fs::write(&out,b"SQLite format 3\0owned-fixture").unwrap();let bytes=std::fs::read(&out).unwrap();let name=out.to_str().unwrap();
        for format in ["csv","html","xlsx","sitemap"] {assert!(crate::export("does-not-need-loading.sqlite",name,format).unwrap_err().to_string().contains("SQLite"));assert_eq!(std::fs::read(&out).unwrap(),bytes);}
        assert!(crate::export_link_rows(&[],name).is_err());assert_eq!(std::fs::read(&out).unwrap(),bytes);
        assert!(check(root.join("new.csv").to_str().unwrap()).is_ok());assert!(!root.join("new.csv").exists());
        let report=root.join("existing.csv");std::fs::write(&report,b"old report").unwrap();assert!(check(report.to_str().unwrap()).is_ok());assert_eq!(std::fs::read(&report).unwrap(),b"old report");
        std::fs::remove_dir_all(&root).unwrap();
    }
}

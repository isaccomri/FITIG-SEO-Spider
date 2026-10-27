//! Windows common file picker. Selection never opens, exports or creates the chosen file.
use std::sync::atomic::{AtomicBool, Ordering};
static BUSY: AtomicBool = AtomicBool::new(false);
pub struct Guard;
impl Guard {
    pub fn acquire() -> Result<Self, String> {
        BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Self).map_err(|_| "A file picker is already open".into())
    }
}
impl Drop for Guard { fn drop(&mut self) { BUSY.store(false, Ordering::SeqCst); } }
pub struct Spec { pub save: bool, pub title: &'static str, pub filter: &'static str, pub extension: &'static str }
pub fn spec(kind: &str) -> Result<Spec, String> {
    let (save,title,filter,extension) = match kind {
        "load_project" => (false,"FITIG - Open saved crawl","SQLite crawl (*.sqlite;*.db)\0*.sqlite;*.db\0All files (*.*)\0*.*\0\0","sqlite"),
        "compare_crawls" => (false,"FITIG - Choose baseline crawl","SQLite crawl (*.sqlite;*.db)\0*.sqlite;*.db\0All files (*.*)\0*.*\0\0","sqlite"),
        "export_csv" | "export_table" | "export_links" => (true,"FITIG - Save CSV export","CSV (*.csv)\0*.csv\0All files (*.*)\0*.*\0\0","csv"),
        "export_html" => (true,"FITIG - Save HTML report","HTML (*.html)\0*.html\0All files (*.*)\0*.*\0\0","html"),
        "export_excel" => (true,"FITIG - Save Excel report","Excel (*.xlsx)\0*.xlsx\0All files (*.*)\0*.*\0\0","xlsx"),
        "export_sitemap" => (true,"FITIG - Save XML sitemap","XML (*.xml)\0*.xml\0All files (*.*)\0*.*\0\0","xml"),
        "export_svg" => (true,"FITIG - Save SVG diagram","SVG (*.svg)\0*.svg\0All files (*.*)\0*.*\0\0","svg"),
        "export_png" => (true,"FITIG - Save PNG image","PNG (*.png)\0*.png\0All files (*.*)\0*.*\0\0","png"),
        "export_zip" => (true,"FITIG - Save sitemap ZIP","ZIP (*.zip)\0*.zip\0All files (*.*)\0*.*\0\0","zip"),
        _ => return Err("Unsupported file picker action".into()),
    }; Ok(Spec {save,title,filter,extension})
}
fn initial(value: &str) -> Result<Vec<u16>, String> {
    let encoded: Vec<u16> = value.encode_utf16().collect();
    if encoded.len() >= 32768 || encoded.contains(&0) { return Err("File picker path is too long or contains a null character".into()); }
    let mut buffer=vec![0;32768]; buffer[..encoded.len()].copy_from_slice(&encoded); Ok(buffer)
}
#[cfg(windows)]
pub fn choose(owner: usize, spec: Spec, value: &str) -> Result<Option<String>, String> {
    use std::ffi::c_void;
    #[repr(C)] struct OpenFileName {
        size:u32, owner:*mut c_void, instance:*mut c_void, filter:*const u16,
        custom_filter:*mut u16, max_custom_filter:u32, filter_index:u32,
        file:*mut u16, max_file:u32, file_title:*mut u16, max_file_title:u32,
        initial_dir:*const u16, title:*const u16, flags:u32, file_offset:u16,
        file_extension:u16, default_extension:*const u16, custom_data:isize,
        hook:usize, template:*const u16, reserved:*mut c_void, reserved_size:u32, flags_ex:u32,
    }
    #[link(name="comdlg32")] extern "system" {
        fn GetOpenFileNameW(options:*mut OpenFileName)->i32;
        fn GetSaveFileNameW(options:*mut OpenFileName)->i32;
        fn CommDlgExtendedError()->u32;
    }
    let mut file=initial(value)?;
    let wide=|text:&str|text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let title=wide(spec.title); let filter=wide(spec.filter); let extension=wide(spec.extension);
    // Explorer UI, existing parent, no recent-doc entry, no write probe and no cwd change.
    // Save confirms overwrites; open requires an existing file. No multiselect/hooks/templates.
    let mut options=OpenFileName {
        size:std::mem::size_of::<OpenFileName>() as u32,owner:owner as *mut c_void,
        instance:std::ptr::null_mut(),filter:filter.as_ptr(),custom_filter:std::ptr::null_mut(),
        max_custom_filter:0,filter_index:1,file:file.as_mut_ptr(),max_file:file.len() as u32,
        file_title:std::ptr::null_mut(),max_file_title:0,initial_dir:std::ptr::null(),title:title.as_ptr(),
        flags:0x00080000|0x00000800|0x00000008|0x02000000|0x00010000|0x00000004|
            if spec.save {0x00000002} else {0x00001000},
        file_offset:0,file_extension:0,default_extension:extension.as_ptr(),custom_data:0,
        hook:0,template:std::ptr::null(),reserved:std::ptr::null_mut(),reserved_size:0,flags_ex:0,
    };
    // Buffers and struct remain alive through the blocking Windows call; all pointers are bounded.
    let accepted=unsafe {if spec.save {GetSaveFileNameW(&mut options)} else {GetOpenFileNameW(&mut options)}};
    if accepted==0 {let error=unsafe {CommDlgExtendedError()};return if error==0 {Ok(None)} else {Err(format!("Windows file picker failed (0x{error:08x})"))};}
    let length=file.iter().position(|value|*value==0).ok_or("Windows returned an unterminated file path")?;
    String::from_utf16(&file[..length]).map(Some).map_err(|_|"Windows returned an invalid Unicode path".into())
}
#[cfg(not(windows))]
pub fn choose(_:usize,_:Spec,_:&str)->Result<Option<String>,String>{Err("Native file picking is available on Windows; enter a full path instead".into())}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn picker_actions_have_explicit_modes_and_extensions() {
        for kind in ["load_project","compare_crawls"] {let value=spec(kind).unwrap();assert!(!value.save);assert_eq!(value.extension,"sqlite");}
        for (kind,extension) in [("export_csv","csv"),("export_table","csv"),("export_links","csv"),("export_html","html"),("export_excel","xlsx"),("export_sitemap","xml"),("export_svg","svg"),("export_png","png"),("export_zip","zip")] {let value=spec(kind).unwrap();assert!(value.save);assert_eq!(value.extension,extension);assert!(value.filter.ends_with("\0\0"));}
        assert!(spec("delete").is_err());
    }
    #[test] fn wide_path_handles_hebrew_and_rejects_truncation_or_nulls() {
        let path="C:\\בדיקה\\סריקה.sqlite";let buffer=initial(path).unwrap();assert_eq!(String::from_utf16(&buffer[..path.encode_utf16().count()]).unwrap(),path);
        assert!(initial("bad\0path").is_err());assert!(initial(&"x".repeat(32768)).is_err());
    }
    #[test] fn only_one_native_picker_can_be_active() {
        let guard=Guard::acquire().unwrap();assert!(Guard::acquire().is_err());drop(guard);assert!(Guard::acquire().is_ok());
    }
}

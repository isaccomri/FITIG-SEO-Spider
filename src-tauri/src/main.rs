#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod renderer;
mod batch_schedule;
mod file_dialog;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager, State};
#[derive(Default)]
struct Runtime { db: String, running: bool, initializing: bool, error: String, queued: usize, mode:String, seed:String, snapshot_key:String, revision:u64, completed:usize }
#[derive(Default)]
struct Workbench(Mutex<Runtime>);
#[derive(Default)]
struct BatchRuntime {phase:String,scheduled_for:Option<u64>,running:bool,cancel:std::sync::Arc<std::sync::atomic::AtomicBool>,report:Option<fitig_engine::batch::Report>,error:String}
#[derive(Default)]
struct BatchWorkbench(Mutex<BatchRuntime>);
#[derive(Default)]struct SheetsGate(std::sync::Arc<std::sync::atomic::AtomicBool>);
struct SheetsPermit(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for SheetsPermit{fn drop(&mut self){self.0.store(false,std::sync::atomic::Ordering::SeqCst)}}
#[tauri::command]
fn preview_google_sheets(input:fitig_engine::google_sheets::Input)->Result<fitig_engine::google_sheets::Plan,String>{fitig_engine::google_sheets::plan(&input).map_err(|e|e.to_string())}
#[tauri::command]
async fn create_google_spreadsheet(state:State<'_,Workbench>,gate:State<'_,SheetsGate>,input:fitig_engine::google_sheets::Input,credential:String,allow_create:bool)->Result<fitig_engine::google_sheets::Receipt,String>{
 {let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before exporting Google Sheets".into())}}
 if gate.0.compare_exchange(false,true,std::sync::atomic::Ordering::SeqCst,std::sync::atomic::Ordering::SeqCst).is_err(){return Err("A Google Sheets export is already in progress".into())}
 let permit=SheetsPermit(gate.0.clone());tauri::async_runtime::spawn_blocking(move||{let _permit=permit;fitig_engine::google_sheets::create(input,&credential,allow_create).map_err(|e|e.to_string())}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn load_semantic_search(state:State<Workbench>)->Result<fitig_engine::semantic::State,String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before reading semantic search".into())}fitig_engine::semantic::load(&r.db).map_err(|e|e.to_string())}
#[tauri::command]
fn import_semantic_corpus(state:State<Workbench>,raw:String)->Result<fitig_engine::semantic::Summary,String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before importing embeddings".into())}fitig_engine::semantic::import(&r.db,&raw).map_err(|e|e.to_string())}
#[tauri::command]
fn search_semantic_vectors(state:State<Workbench>,query:fitig_engine::semantic::Query)->Result<fitig_engine::semantic::Report,String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before searching embeddings".into())}fitig_engine::semantic::search(&r.db,query).map_err(|e|e.to_string())}
#[tauri::command]
fn validate_batch(manifest:fitig_engine::batch::Manifest)->Result<fitig_engine::batch::Manifest,String>{fitig_engine::batch::validate(manifest).map_err(|error|error.to_string())}
#[tauri::command]
fn get_batch(state:State<BatchWorkbench>)->Result<Value,String>{let runtime=state.0.lock().map_err(|error|error.to_string())?;Ok(json!({"running":runtime.running,"report":runtime.report,"error":runtime.error,"phase":runtime.phase,"scheduled_for":runtime.scheduled_for}))}
#[tauri::command]
fn stop_batch(state:State<BatchWorkbench>)->Result<(),String>{let runtime=state.0.lock().map_err(|error|error.to_string())?;runtime.cancel.store(true,std::sync::atomic::Ordering::Relaxed);Ok(())}
#[tauri::command]
fn start_batch(app:tauri::AppHandle,manifest:fitig_engine::batch::Manifest,output:String,run_at:Option<u64>)->Result<(),String>{
 batch_schedule::validate(run_at,batch_schedule::now()?)?;
 let manifest=fitig_engine::batch::validate(manifest).map_err(|error|error.to_string())?;fitig_engine::batch::output_path(&output).map_err(|error|error.to_string())?;
 let state=app.state::<Workbench>();let workbench=state.0.lock().map_err(|error|error.to_string())?;if workbench.running{return Err("Stop the interactive crawl before starting a batch".into())}
 let batch=app.state::<BatchWorkbench>();let mut runtime=batch.0.lock().map_err(|error|error.to_string())?;if runtime.running{return Err("A batch is already running".into())}let cancel=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));runtime.cancel=cancel.clone();runtime.running=true;runtime.phase=if run_at.is_some(){"scheduled"}else{"running"}.into();runtime.scheduled_for=run_at;runtime.error.clear();runtime.report=None;drop(runtime);drop(workbench);
 std::thread::spawn(move||{
 let ready=batch_schedule::wait(run_at,&cancel);
 {let state=app.state::<BatchWorkbench>();if let Ok(mut runtime)=state.0.lock(){
   match ready{Ok(true)=>runtime.phase="running".into(),Ok(false)=>{runtime.running=false;runtime.phase="cancelled".into();return},Err(error)=>{runtime.running=false;runtime.phase="failed".into();runtime.error=error;return}}
 };}
 let progress_app=app.clone();let result=fitig_engine::batch::run(manifest,&output,cancel,move|report|{let state=progress_app.state::<BatchWorkbench>();if let Ok(mut runtime)=state.0.lock(){runtime.report=Some(report.clone());};});let state=app.state::<BatchWorkbench>();if let Ok(mut runtime)=state.0.lock(){runtime.running=false;match result{Ok(report)=>{runtime.phase=report.status.clone();runtime.report=Some(report)},Err(error)=>{runtime.phase="failed".into();runtime.error=error.to_string()}}};});Ok(())
}

#[tauri::command]
async fn choose_file_path(app:tauri::AppHandle,kind:String,initial:Option<String>)->Result<Option<String>,String>{
 let spec=file_dialog::spec(&kind)?;let guard=file_dialog::Guard::acquire()?;
 #[cfg(windows)] let owner=app.get_webview_window("main").ok_or("Workbench window unavailable")?.hwnd().map_err(|error|error.to_string())?.0 as usize;
 #[cfg(not(windows))] let owner={let _=app;0};
 tauri::async_runtime::spawn_blocking(move||{let _guard=guard;file_dialog::choose(owner,spec,&initial.unwrap_or_default())}).await.map_err(|error|error.to_string())?
}

fn project_stamp(db:&str)->String{if db.is_empty(){return String::new()}let stat=|path:&str|std::fs::metadata(path).ok().map(|metadata|(metadata.len(),metadata.modified().ok()));format!("{:?}",(stat(db),stat(&format!("{db}-wal"))))}
fn runtime_snapshot_key(runtime:&Runtime)->String{format!("{:?}",(&runtime.db,runtime.running,runtime.initializing,&runtime.error,runtime.queued,&runtime.mode,&runtime.seed,project_stamp(&runtime.db)))}
fn snapshot(state:&Workbench)->Result<Value,String>{snapshot_since(state,None)}
fn snapshot_since(state:&Workbench,revision:Option<u64>)->Result<Value,String>{
 let mut r=state.0.lock().map_err(|e|e.to_string())?;
 let key=runtime_snapshot_key(&r);
 let changed=r.snapshot_key!=key;
 if changed{r.revision=r.revision.wrapping_add(1);r.snapshot_key=key;}
 let mut view=json!({"running":r.running,"error":r.error,"queued":r.queued,"project_path":r.db,"mode":r.mode,"seed":r.seed,"revision":r.revision,"completed":r.completed});
 if !changed&&revision==Some(r.revision){view["unchanged"]=json!(true);return Ok(view)}
 let raw=if r.initializing||r.db.is_empty()||!std::path::Path::new(&r.db).exists(){Vec::new()}else{match fitig_engine::load_pages(&r.db){Ok(pages)=>pages,Err(error)=>{r.snapshot_key.clear();return Err(error.to_string())}}};
 r.completed=raw.len();view["completed"]=json!(r.completed);view["pages"]=json!(raw);view["unchanged"]=json!(false);Ok(view)
}
#[tauri::command]
fn get_snapshot(state:State<Workbench>,revision:Option<u64>)->Result<Value,String>{snapshot_since(&state,revision)}
#[tauri::command]
async fn analyze_link_graph(app:tauri::AppHandle)->Result<Value,String>{
 let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before link analysis".into())}runtime.db.clone()};
 let analysis_db=db.clone();let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::link_analysis::project(&analysis_db).map_err(|error|error.to_string())).await.map_err(|error|error.to_string())??;
 let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.db!=db||runtime.running{return Err("Project changed during link analysis; reopen the report".into())}
 serde_json::to_value(report).map_err(|error|error.to_string())
}
#[tauri::command]
async fn analyze_saved_text(app:tauri::AppHandle,kind:String,options:fitig_engine::text_analysis::Options)->Result<Value,String>{
 if !["ngrams","duplicates"].contains(&kind.as_str()){return Err("Unknown saved-text analysis".into())}
 let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before text analysis".into())}runtime.db.clone()};
 let analysis_db=db.clone();let report=tauri::async_runtime::spawn_blocking(move||->Result<Value,String>{if kind=="ngrams"{serde_json::to_value(fitig_engine::text_analysis::ngrams(&analysis_db,options).map_err(|error|error.to_string())?).map_err(|error|error.to_string())}else{serde_json::to_value(fitig_engine::text_analysis::duplicates(&analysis_db,options).map_err(|error|error.to_string())?).map_err(|error|error.to_string())}}).await.map_err(|error|error.to_string())??;
 let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.db!=db||runtime.running{return Err("Project changed during text analysis; reopen the report".into())}Ok(report)
}
#[tauri::command]
fn validate_form_authentication(profile:fitig_engine::CrawlProfile,credentials:Option<fitig_engine::form_auth::Credentials>)->Result<(),String>{let seed=profile.form_auth.as_ref().map(|c|c.login_url.as_str()).unwrap_or("https://example.test/");fitig_engine::form_auth::validate(&profile,credentials.as_ref(),seed).map_err(|e|e.to_string())}
#[tauri::command]
fn start_crawl(app:tauri::AppHandle,state:State<Workbench>,url:String,limit:usize,delay_ms:u64,path:Option<String>,resume:Option<bool>,mode:Option<String>,urls:Option<Vec<String>>,profile:Option<fitig_engine::CrawlProfile>,authentication:Option<fitig_engine::authentication::Session>,proxy_authentication:Option<fitig_engine::authentication::Session>,form_credentials:Option<fitig_engine::form_auth::Credentials>)->Result<(),String>{
 let profile=profile.unwrap_or_default();fitig_engine::form_auth::validate(&profile,form_credentials.as_ref(),&url).map_err(|e|e.to_string())?;fitig_engine::validate_request_timeout(profile.request_timeout_secs).map_err(|error|error.to_string())?;fitig_engine::proxy_settings::validate(&profile,proxy_authentication.as_ref()).map_err(|error|error.to_string())?;fitig_engine::http_cookies::validate(profile.cookies_enabled,profile.rendering.enabled).map_err(|error|error.to_string())?;fitig_engine::authentication::validate_configuration(&profile.http_auth_origin,authentication.as_ref(),profile.rendering.enabled).map_err(|error|error.to_string())?;fitig_engine::rewriting::Program::compile(&profile.url_rewriting).map_err(|error|error.to_string())?;fitig_engine::render::validate(&profile.rendering).map_err(|e|e.to_string())?;fitig_engine::CustomProgram::compile(&profile.custom_rules).map_err(|e|e.to_string())?;
 let mode=mode.unwrap_or_else(||"spider".into()).to_ascii_lowercase();if !["spider","list"].contains(&mode.as_str()){return Err("Mode must be Spider or List".into())}if mode=="list"&&urls.as_ref().map(|v|v.is_empty()).unwrap_or(true){return Err("Enter URLs for List mode first".into())}
 let mut r=state.0.lock().map_err(|e|e.to_string())?;if r.running{return Err("A crawl is already running".into())}let batch=app.state::<BatchWorkbench>();let batch_guard=batch.0.lock().map_err(|error|error.to_string())?;if batch_guard.running{return Err("Stop the batch before starting an interactive crawl".into())}
 let db=if let Some(path)=path.filter(|p|!p.trim().is_empty()){path}else if resume.unwrap_or(false)&&!r.db.is_empty(){r.db.clone()}else{let folder=app.path().app_data_dir().map_err(|e|e.to_string())?;std::fs::create_dir_all(&folder).map_err(|e|e.to_string())?;folder.join(format!("crawl-{}.sqlite",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis())).to_string_lossy().into_owned()};
 if std::path::Path::new(&db).exists()&&!resume.unwrap_or(false){return Err("Project exists. Choose Resume or a new database path to preserve its data.".into())}
 let stop=PathBuf::from(format!("{db}.stop"));if stop.exists(){std::fs::remove_file(stop).map_err(|e|e.to_string())?}
 r.db=db.clone();r.running=true;r.initializing=true;r.mode=mode.clone();r.seed=url.clone();r.error.clear();r.queued=0;drop(r);drop(batch_guard);drop(batch);
 std::thread::spawn(move||{let options=fitig_engine::CrawlOptions{url,db,limit,delay_ms:delay_ms.min(60000),resume:resume.unwrap_or(false)};let progress=|p:fitig_engine::Progress|{let state=app.state::<Workbench>();if let Ok(mut r)=state.0.lock(){r.queued=p.queued;r.completed=p.completed;r.initializing=false;r.snapshot_key.clear();};};let user_agent=profile.user_agent.clone();let result=fitig_engine::crawl_with_sessions_and_renderer(options,if mode=="list"{urls}else{None},profile,authentication,proxy_authentication,form_credentials,|url,options|renderer::render(&app,url,options,&user_agent),progress);let state=app.state::<Workbench>();if let Ok(mut r)=state.0.lock(){r.running=false;r.initializing=false;if let Err(e)=result{r.error=e.to_string()}}if let Ok(s)=snapshot(&state){let _=app.emit("crawl-update",s);}});Ok(())
}
#[tauri::command]
fn stop_crawl(state:State<Workbench>)->Result<(),String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.running{std::fs::write(format!("{}.stop",r.db),b"stop").map_err(|e|e.to_string())?}Ok(())}
#[tauri::command]
async fn load_project(app:tauri::AppHandle,path:String)->Result<Value,String>{{let batch=app.state::<BatchWorkbench>();if batch.0.lock().map_err(|error|error.to_string())?.running{return Err("Stop the batch before loading a project".into())}}
 let state=app.state::<Workbench>();let previous={let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running{return Err("Stop the running crawl first".into())}runtime.db.clone()};
 let input=path.clone();let(mut report,stamp,stable)=tauri::async_runtime::spawn_blocking(move||->Result<_,String>{let before=project_stamp(&input);let project=fitig_engine::load_project_data(&input).map_err(|error|error.to_string())?;let stamp=project_stamp(&input);let stable=before==stamp;let report=serde_json::to_value(project).map_err(|error|error.to_string())?;Ok((report,stamp,stable))}).await.map_err(|error|error.to_string())??;
 let mut runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db!=previous{return Err("Project changed while loading; the selected project was not replaced".into())}if app.state::<BatchWorkbench>().0.lock().map_err(|error|error.to_string())?.running{return Err("A batch started while loading; the project was not replaced".into())}
 runtime.db=path;runtime.seed=report["seed"].as_str().unwrap_or("").into();runtime.mode=report["mode"].as_str().unwrap_or("spider").into();runtime.queued=report["queued"].as_u64().unwrap_or(0)as usize;runtime.completed=report["pages"].as_array().map_or(0,|pages|pages.len());runtime.initializing=false;runtime.error.clear();runtime.revision=runtime.revision.wrapping_add(1);runtime.snapshot_key=if stable&&stamp==project_stamp(&runtime.db){runtime_snapshot_key(&runtime)}else{String::new()};
 report["running"]=json!(false);report["error"]=json!("");report["project_path"]=json!(runtime.db);report["revision"]=json!(runtime.revision);report["completed"]=json!(runtime.completed);report["unchanged"]=json!(false);Ok(report)
}
fn current_db(state:&Workbench)->Result<String,String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.db.is_empty(){Err("Open or start a crawl first".into())}else{Ok(r.db.clone())}}
#[tauri::command]
fn export_csv(state:State<Workbench>,path:String)->Result<(),String>{fitig_engine::export(&current_db(&state)?,&path,"csv").map_err(|e|e.to_string())}
#[tauri::command]
fn export_report(state:State<Workbench>,path:String,format:String)->Result<(),String>{fitig_engine::export(&current_db(&state)?,&path,&format).map_err(|e|e.to_string())}
#[tauri::command]
fn export_link_rows(path:String,rows:Vec<fitig_engine::LinkReportRow>)->Result<(),String>{fitig_engine::export_link_rows(&rows,&path).map_err(|e|e.to_string())}
#[tauri::command]
fn analyze_crawl(state:State<Workbench>)->Result<Value,String>{fitig_engine::analyze(&current_db(&state)?).map_err(|e|e.to_string())}
#[tauri::command]
async fn compare_crawls(app:tauri::AppHandle,path:String)->Result<Value,String>{
 let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped current crawl before comparison".into())}runtime.db.clone()};let current=db.clone();
 let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::crawl_compare::projects(&current,&path).map_err(|error|error.to_string())).await.map_err(|error|error.to_string())??;
 let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db!=db{return Err("Project changed during comparison; reopen the report".into())}serde_json::to_value(report).map_err(|error|error.to_string())
}
#[tauri::command]
fn export_table(path:String,headers:Vec<String>,rows:Vec<Vec<String>>)->Result<(),String>{fitig_engine::export_target::check(&path).map_err(|error|error.to_string())?;if std::path::Path::new(&path).extension().and_then(|v|v.to_str()).map(|s|s.eq_ignore_ascii_case("csv"))!=Some(true){return Err("Choose a .csv file".into())}if headers.is_empty()||rows.iter().any(|row|row.len()!=headers.len()){return Err("Table rows must match headers".into())}let mut bytes=vec![0xef,0xbb,0xbf];{let mut writer=csv::Writer::from_writer(&mut bytes);writer.write_record(headers).map_err(|e|e.to_string())?;for row in rows{writer.write_record(row).map_err(|e|e.to_string())?}writer.flush().map_err(|e|e.to_string())?;}std::fs::write(path,bytes).map_err(|e|e.to_string())}
#[tauri::command]
fn export_visualisation(path:String,content:String,format:String)->Result<(),String>{fitig_engine::export_target::check(&path).map_err(|error|error.to_string())?;if format=="png"{return fitig_engine::chart_export::export(&path,&content).map_err(|error|error.to_string())}if !["svg","html"].contains(&format.as_str()){return Err("Visualisation format must be svg, html or png".into())}let extension=std::path::Path::new(&path).extension().and_then(|v|v.to_str()).unwrap_or("");if !extension.eq_ignore_ascii_case(&format){return Err(format!("Choose a .{format} file"))}if content.len()>32*1024*1024{return Err("Visualisation exceeds 32 MiB export limit".into())}std::fs::write(path,content.as_bytes()).map_err(|e|e.to_string())}
#[tauri::command]
fn validate_custom_rules(rules:Vec<fitig_engine::CustomRule>)->Result<(),String>{fitig_engine::CustomProgram::compile(&rules).map(|_|()).map_err(|e|e.to_string())}
#[tauri::command]
fn plan_link_metrics(endpoint:String,date:String,mode:String,urls:Vec<String>)->Result<fitig_engine::link_metrics::Plan,String>{fitig_engine::link_metrics::plan(&endpoint,&date,&mode,&urls).map_err(|e|e.to_string())}
#[tauri::command]
fn load_link_metrics(state:State<Workbench>)->Result<Option<fitig_engine::link_metrics::Report>,String>{fitig_engine::link_metrics::load(&current_db(&state)?).map_err(|e|e.to_string())}
#[tauri::command]
fn import_link_metrics(state:State<Workbench>,endpoint:String,date:String,mode:String,urls:Vec<String>,response_json:String)->Result<fitig_engine::link_metrics::Report,String>{if response_json.len()>1024*1024{return Err("Ahrefs import exceeds 1 MiB".into())}let response:Value=serde_json::from_str(&response_json).map_err(|_|"Invalid Ahrefs JSON; response body omitted")?;let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before importing link metrics".into())}let report=fitig_engine::link_metrics::import(&endpoint,&date,&mode,&urls,&response).map_err(|e|e.to_string())?;fitig_engine::link_metrics::save(&r.db,&report).map_err(|e|e.to_string())?;Ok(report)}
#[tauri::command]
async fn query_link_metrics(app:tauri::AppHandle,request:fitig_engine::link_metrics::Request)->Result<fitig_engine::link_metrics::Report,String>{let state=app.state::<Workbench>();let db={let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before requesting link metrics".into())}r.db.clone()};let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::link_metrics::query(request).map_err(|e|e.to_string())).await.map_err(|e|e.to_string())??;let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db!=db{return Err("Project changed during link metrics request; report not saved".into())}fitig_engine::link_metrics::save(&db,&report).map_err(|e|e.to_string())?;Ok(report)}
#[tauri::command]
async fn spelling_languages()->Result<Vec<String>,String>{tauri::async_runtime::spawn_blocking(fitig_engine::spelling::languages).await.map_err(|e|e.to_string())?}
#[tauri::command]
async fn check_spelling(app:tauri::AppHandle,url:String,language:String)->Result<fitig_engine::spelling::Report,String>{let state=app.state::<Workbench>();let db={let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before checking spelling".into())}r.db.clone()};let source=db.clone();let report=tauri::async_runtime::spawn_blocking(move||{let text=fitig_engine::spelling::source(&source,&url).map_err(|e|e.to_string())?;fitig_engine::spelling::check(&text,&language)}).await.map_err(|e|e.to_string())??;let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db!=db{return Err("Project changed during spelling analysis".into())}Ok(report)}
#[tauri::command]
async fn validate_amp(app:tauri::AppHandle,url:String)->Result<fitig_engine::amp::Report,String>{
 let state=app.state::<Workbench>();let db={let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db.is_empty(){return Err("Open a stopped crawl before AMP validation".into())}r.db.clone()};let source=db.clone();let host=app.clone();
 let report=tauri::async_runtime::spawn_blocking(move||{let html=fitig_engine::amp::source(&source,&url).map_err(|e|e.to_string())?;renderer::validate_amp(&host,&html)}).await.map_err(|e|e.to_string())??;
 let r=state.0.lock().map_err(|e|e.to_string())?;if r.running||r.db!=db{return Err("Project changed during AMP validation".into())}Ok(report)
}
#[tauri::command]
async fn analyze_pagination(app:tauri::AppHandle)->Result<fitig_engine::pagination::Report,String>{let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before analyzing pagination".into())}runtime.db.clone()};let source=db.clone();let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::pagination::analyze(&source).map_err(|e|e.to_string())).await.map_err(|e|e.to_string())??;let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db!=db{return Err("Project changed during pagination analysis".into())}Ok(report)}
#[tauri::command]
async fn analyze_structured_data(app:tauri::AppHandle,options:fitig_engine::structured_data::Options)->Result<fitig_engine::structured_data::Report,String>{
 let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before validating structured data".into())}runtime.db.clone()};let source=db.clone();
 let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::structured_data::analyze(&source,options).map_err(|e|e.to_string())).await.map_err(|e|e.to_string())??;
 let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db!=db{return Err("Project changed during structured data validation".into())}Ok(report)
}
#[tauri::command]
fn get_json_ld_source(state:State<Workbench>,url:String,block:usize)->Result<String,String>{fitig_engine::structured_data::source(&current_db(&state)?,&url,block).map_err(|e|e.to_string())}
#[tauri::command]
fn get_source(state:State<Workbench>,url:String,rendered:Option<bool>)->Result<Option<String>,String>{let db=current_db(&state)?;if rendered.unwrap_or(false){fitig_engine::load_rendered_source(&db,&url)}else{fitig_engine::load_source(&db,&url)}.map_err(|e|e.to_string())}
#[tauri::command]
fn validate_proxy_settings(profile:fitig_engine::CrawlProfile,proxy_authentication:Option<fitig_engine::authentication::Session>)->Result<String,String>{fitig_engine::proxy_settings::validate(&profile,proxy_authentication.as_ref()).map_err(|error|error.to_string())?;if profile.proxy_mode=="manual"{fitig_engine::authentication::normalize_origin(&profile.proxy_url).map_err(|error|error.to_string())}else{Ok(String::new())}}
#[tauri::command]
fn get_http_cookies(state:State<Workbench>)->Result<Option<fitig_engine::http_cookies::Report>,String>{fitig_engine::http_cookies::report(&current_db(&state)?).map_err(|error|error.to_string())}
#[tauri::command]
fn clear_http_cookies(state:State<Workbench>)->Result<Option<fitig_engine::http_cookies::Report>,String>{let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running{return Err("Stop the crawl before clearing cookies".into())}if runtime.db.is_empty(){return Err("Open a saved crawl first".into())}fitig_engine::http_cookies::clear(&runtime.db).map_err(|error|error.to_string())}
#[tauri::command]
fn validate_http_authentication(origin:String,authentication:Option<fitig_engine::authentication::Session>)->Result<String,String>{fitig_engine::authentication::validate_configuration(&origin,authentication.as_ref(),false).map_err(|error|error.to_string())?;if origin.is_empty(){Ok(origin)}else{fitig_engine::authentication::normalize_origin(&origin).map_err(|error|error.to_string())}}
#[tauri::command]
fn preview_url_rewriting(options:fitig_engine::rewriting::Options,urls:Vec<String>)->Result<Vec<fitig_engine::rewriting::Entry>,String>{fitig_engine::rewriting::preview(&options,&urls).map_err(|error|error.to_string())}
#[tauri::command]
fn get_url_rewrite_log(state:State<Workbench>,url:String)->Result<Option<fitig_engine::rewriting::Report>,String>{let db=current_db(&state)?;fitig_engine::rewriting::load(&db,&url).map_err(|error|error.to_string())}
#[tauri::command]
fn validate_browser_scripts(rules:Vec<fitig_engine::javascript::Rule>)->Result<(),String>{fitig_engine::javascript::validate(&rules).map_err(|error|error.to_string())}
#[tauri::command]
fn get_javascript_results(state:State<Workbench>,url:String)->Result<Option<fitig_engine::javascript::Report>,String>{fitig_engine::javascript::load(&current_db(&state)?,&url).map_err(|error|error.to_string())}
#[tauri::command]
fn get_mobile(state:State<Workbench>,url:String)->Result<Option<fitig_engine::mobile::Report>,String>{fitig_engine::mobile::load(&current_db(&state)?,&url).map_err(|error|error.to_string())}
#[tauri::command]
fn get_accessibility(state:State<Workbench>,url:String)->Result<Option<fitig_engine::accessibility::Report>,String>{fitig_engine::accessibility::load(&current_db(&state)?,&url).map_err(|error|error.to_string())}
#[tauri::command]
fn get_console_log(state:State<Workbench>,url:String)->Result<Option<fitig_engine::browser_logs::Report>,String>{fitig_engine::browser_logs::load(&current_db(&state)?,&url).map_err(|error|error.to_string())}
#[tauri::command]
fn get_screenshot(state:State<Workbench>,url:String)->Result<Option<Value>,String>{let bytes=fitig_engine::screenshots::load(&current_db(&state)?,&url).map_err(|error|error.to_string())?;bytes.map(|bytes|{let(width,height)=fitig_engine::screenshots::dimensions(&bytes).map_err(|error|error.to_string())?;Ok(json!({"width":width,"height":height,"data_url":fitig_engine::screenshots::data_url(&bytes).map_err(|error|error.to_string())?}))}).transpose()}
#[tauri::command]
fn export_screenshot(state:State<Workbench>,url:String,path:String)->Result<(),String>{fitig_engine::screenshots::export(&current_db(&state)?,&url,&path).map_err(|error|error.to_string())}
#[tauri::command]
fn apply_custom_rules(state:State<Workbench>,rules:Vec<fitig_engine::CustomRule>)->Result<usize,String>{let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running{return Err("Stop the crawl before reprocessing stored HTML".into())}if runtime.db.is_empty(){return Err("Open a project first".into())}fitig_engine::reprocess_custom(&runtime.db,rules).map_err(|e|e.to_string())}
#[tauri::command]
async fn export_sitemap_bundle(app:tauri::AppHandle,path:String,options:fitig_engine::sitemap_export::Options)->Result<fitig_engine::sitemap_export::Report,String>{let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|error|error.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before exporting a sitemap package".into())}runtime.db.clone()};tauri::async_runtime::spawn_blocking(move||fitig_engine::sitemap_export::export(&db,&path,options).map_err(|error|error.to_string())).await.map_err(|error|error.to_string())?}
#[tauri::command]
fn get_sitemap_inventory(state:State<Workbench>)->Result<Option<Value>,String>{fitig_engine::project_sitemaps(&current_db(&state)?).map_err(|e|e.to_string())}
#[tauri::command]
async fn discover_sitemaps(app:tauri::AppHandle)->Result<fitig_engine::sitemaps::Discovery,String>{
 let state=app.state::<Workbench>();let db={let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before discovering sitemaps".into())}runtime.db.clone()};
 let seed=fitig_engine::project_seed(&db).map_err(|e|e.to_string())?;let profile=fitig_engine::project_profile(&db).map_err(|e|e.to_string())?;
 let report=tauri::async_runtime::spawn_blocking(move||fitig_engine::sitemaps::discover(&seed,&profile.user_agent).map_err(|e|e.to_string())).await.map_err(|e|e.to_string())??;
 let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db!=db{return Err("Project changed during robots discovery".into())}Ok(report)
}
#[tauri::command]
async fn retrieve_sitemaps(app:tauri::AppHandle,url:String,urls:Option<Vec<String>>)->Result<Value,String>{
 let state=app.state::<Workbench>();
 let db={let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl before retrieving sitemaps".into())}runtime.db.clone()};
 let profile=fitig_engine::project_profile(&db).map_err(|e|e.to_string())?;
 let inventory=tauri::async_runtime::spawn_blocking(move||match urls{Some(urls)=>fitig_engine::sitemaps::retrieve_many(&urls,&profile.user_agent),None=>fitig_engine::sitemaps::retrieve(&url,&profile.user_agent)}.map_err(|e|e.to_string())).await.map_err(|e|e.to_string())??;
 let value=serde_json::to_value(inventory).map_err(|e|e.to_string())?;
 let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db!=db{return Err("Project changed during retrieval; sitemap inventory not saved".into())}
 fitig_engine::save_sitemaps(&db,value.clone()).map_err(|e|e.to_string())?;Ok(value)
}
#[tauri::command]
fn save_sitemap_inventory(state:State<Workbench>,inventory:Value)->Result<(),String>{let r=state.0.lock().map_err(|e|e.to_string())?;if r.running{return Err("Stop the crawl before importing sitemap membership".into())}if r.db.is_empty(){return Err("Open a crawl before importing sitemap membership".into())}fitig_engine::save_sitemaps(&r.db,inventory).map_err(|e|e.to_string())}
#[tauri::command]
fn load_google_report(state:State<Workbench>,service:String)->Result<Option<fitig_engine::integrations::ApiReport>,String>{fitig_engine::integrations::load_report(&current_db(&state)?,&service).map_err(|e|e.to_string())}
#[tauri::command]
fn google_credential_status(service:String)->Result<bool,String>{fitig_engine::credential_vault::Store::default().exists(&service)}
#[tauri::command]
fn save_google_credential(service:String,credential:String)->Result<(),String>{fitig_engine::credential_vault::Store::default().save(&service,&credential)}
#[tauri::command]
fn remove_google_credential(service:String)->Result<(),String>{fitig_engine::credential_vault::Store::default().remove(&service)}
#[tauri::command]
async fn query_google(app:tauri::AppHandle,mut request:fitig_engine::integrations::ApiRequest,use_saved_credential:Option<bool>)->Result<fitig_engine::integrations::ApiReport,String>{let state=app.state::<Workbench>();let db=current_db(&state)?;let seed=fitig_engine::project_seed(&db).map_err(|e|e.to_string())?;if state.0.lock().map_err(|e|e.to_string())?.running{return Err("Stop the crawl before loading API reports".into())}let report=tauri::async_runtime::spawn_blocking(move||{if use_saved_credential.unwrap_or(false){if !request.credential.is_empty(){return Err("Choose a saved credential or an in-memory credential, not both".into())}request.credential=fitig_engine::credential_vault::Store::default().read(&request.service)?.ok_or("No saved credential for this Google service")?;}fitig_engine::integrations::query(request,&seed).map_err(|e|e.to_string())}).await.map_err(|e|e.to_string())??;let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.db!=db||runtime.running{return Err("Project changed while API request was running; report not saved".into())}fitig_engine::integrations::save_report(&db,&report).map_err(|e|e.to_string())?;Ok(report)}
#[tauri::command]
fn import_google_response(state:State<Workbench>,service:String,property:String,start:String,end:String,strategy:String,response:Value,urls:Option<Vec<String>>)->Result<fitig_engine::integrations::ApiReport,String>{let runtime=state.0.lock().map_err(|e|e.to_string())?;if runtime.running||runtime.db.is_empty(){return Err("Open a stopped crawl first".into())}let seed=fitig_engine::project_seed(&runtime.db).map_err(|e|e.to_string())?;let rows=if service=="inspection"{fitig_engine::url_inspection::import(&property,&urls.unwrap_or_default(),&strategy,&response)}else{fitig_engine::integrations::parse_rows(&service,&response,&seed)}.map_err(|e|e.to_string())?;let report=fitig_engine::integrations::ApiReport{service:service.clone(),property,start:if service=="inspection"{String::new()}else{start},end:if service=="inspection"{String::new()}else{end},strategy,rows,truncated:false,note:if service=="inspection"{"Imported Google-index snapshot response, not a live URL test. URL attribution uses the single explicit URL you supplied because the official response omits it. No live account connection was used. Missing fields are unknown. Detailed AMP/rich-result issues and deprecated mobile usability are not imported.".into()}else{"Imported official API response JSON. No live account connection was used for this import. Missing URLs are unknown, not zero.".into()}};fitig_engine::integrations::save_report(&runtime.db,&report).map_err(|e|e.to_string())?;Ok(report)}
fn main(){let handler:fn(tauri::ipc::Invoke<tauri::Wry>)->bool=tauri::generate_handler![validate_form_authentication,preview_google_sheets,create_google_spreadsheet,load_semantic_search,import_semantic_corpus,search_semantic_vectors,plan_link_metrics,load_link_metrics,import_link_metrics,query_link_metrics,spelling_languages,check_spelling,google_credential_status,save_google_credential,remove_google_credential,validate_amp,get_mobile,get_accessibility,choose_file_path,validate_batch,get_batch,start_batch,stop_batch,validate_proxy_settings,get_http_cookies,clear_http_cookies,validate_http_authentication,preview_url_rewriting,get_url_rewrite_log,validate_browser_scripts,get_javascript_results,analyze_pagination,analyze_structured_data,get_json_ld_source,discover_sitemaps,export_sitemap_bundle,get_console_log,get_screenshot,export_screenshot,analyze_saved_text,analyze_link_graph,retrieve_sitemaps,load_google_report,query_google,import_google_response,get_sitemap_inventory,save_sitemap_inventory,validate_custom_rules,get_source,apply_custom_rules,get_snapshot,start_crawl,stop_crawl,load_project,export_csv,export_report,export_link_rows,export_table,export_visualisation,analyze_crawl,compare_crawls];tauri::Builder::default().manage(Workbench::default()).manage(BatchWorkbench::default()).manage(SheetsGate::default()).invoke_handler(move |invoke|{if invoke.message.webview_ref().label()!="main"{invoke.resolver.reject("Workbench commands are unavailable to rendered websites");return true;}handler(invoke)}).run(tauri::generate_context!()).expect("Unable to run FITIG SEO Spider");}

#[cfg(test)]mod tests{use super::*;#[test]fn initializing_snapshot_does_not_query_unready_database(){let path=std::env::temp_dir().join(format!("fitig-initializing-{}.sqlite",std::process::id()));let file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();drop(file);let state=Workbench(Mutex::new(Runtime{db:path.to_string_lossy().into_owned(),running:true,initializing:true,..Default::default()}));let view=snapshot(&state).unwrap();assert!(view["pages"].as_array().unwrap().is_empty());assert_eq!(view["running"],true);state.0.lock().unwrap().initializing=false;assert!(snapshot(&state).is_err());std::fs::remove_file(path).unwrap();}}

#[cfg(test)]mod visual_export_tests{use super::*;#[test]fn writes_unicode_svg_and_rejects_wrong_extensions(){let path=std::env::temp_dir().join(format!("fitig-visual-export-{}.svg",std::process::id()));let name=path.to_string_lossy().into_owned();export_visualisation(name.clone(),"<svg>שלום</svg>".into(),"svg".into()).unwrap();assert_eq!(std::fs::read_to_string(&path).unwrap(),"<svg>שלום</svg>");assert!(export_visualisation(name.clone(),"bad".into(),"html".into()).is_err());assert!(export_visualisation(name,"bad".into(),"exe".into()).is_err());std::fs::remove_file(path).unwrap();}}

#[cfg(test)]mod table_export_tests{use super::*;#[test]fn exports_filtered_table_with_utf8_bom_and_validates_dimensions(){let path=std::env::temp_dir().join(format!("fitig-table-{}.csv",std::process::id()));let filename=path.to_string_lossy().into_owned();export_table(filename.clone(),vec!["Address".into(),"Status".into()],vec![vec!["https://e.test/שלום?a=1&b=2".into(),"404".into()]]).unwrap();let bytes=std::fs::read(&path).unwrap();assert!(bytes.starts_with(&[0xef,0xbb,0xbf]));assert!(String::from_utf8(bytes).unwrap().contains("שלום"));assert!(export_table(filename,vec!["Header".into()],vec![vec![]]).is_err());std::fs::remove_file(path).unwrap();}}

#[cfg(test)]mod snapshot_revision_tests{
 use super::*;
 #[test]fn unchanged_snapshot_omits_pages_and_metadata_changes_invalidate_revision(){
  let state=Workbench::default();let first=snapshot(&state).unwrap();let revision=first["revision"].as_u64().unwrap();assert!(first["pages"].is_array());
  let cached=snapshot_since(&state,Some(revision)).unwrap();assert_eq!(cached["unchanged"],true);assert!(cached.get("pages").is_none());
  state.0.lock().unwrap().error="changed".into();let changed=snapshot_since(&state,Some(revision)).unwrap();assert_eq!(changed["unchanged"],false);assert!(changed["revision"].as_u64().unwrap()>revision);assert_eq!(changed["error"],"changed");
  assert!(snapshot(&state).unwrap()["pages"].is_array());
 }
}

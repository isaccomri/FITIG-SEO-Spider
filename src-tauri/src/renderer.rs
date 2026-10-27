//! Read the DOM through WebView2's native ExecuteScript result callback.
//! Remote pages receive no workbench capabilities or additional IPC commands.
use std::{sync::mpsc,time::Duration};
use tauri::Manager;
/// Validate a string in an owned blank browser. No saved HTML is navigated or inserted.
pub fn validate_amp(app:&tauri::AppHandle,source:&str)->Result<fitig_engine::amp::Report,String>{
 #[cfg(not(windows))]{let _=(app,source);return Err("AMP validation requires Windows WebView2".into())}
 #[cfg(windows)]{
  use std::sync::atomic::{AtomicBool,Ordering};static ACTIVE:AtomicBool=AtomicBool::new(false);
  if ACTIVE.swap(true,Ordering::SeqCst){return Err("AMP validator is already running".into())}
  struct Guard;impl Drop for Guard{fn drop(&mut self){ACTIVE.store(false,Ordering::SeqCst)}}let _guard=Guard;
  if source.len()>fitig_engine::amp::MAX_SOURCE{return Err("AMP source exceeds 2 MiB validation limit".into())}
  let label=format!("amp-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());let name=label.clone();let host=app.clone();let(tx,rx)=mpsc::channel();
  app.run_on_main_thread(move||{let result=tauri::WebviewWindowBuilder::new(&host,&name,tauri::WebviewUrl::External("about:blank".parse().unwrap())).visible(false).focused(false).skip_taskbar(true)
   .data_directory(std::env::temp_dir().join(format!("fitig-amp-{}",std::process::id())))
   .on_new_window(|_,_|tauri::webview::NewWindowResponse::Deny).on_download(|_,_|false).on_navigation(|url|url.as_str()=="about:blank").build().map(|_|()).map_err(|e|e.to_string());let _=tx.send(result);}).map_err(|e|e.to_string())?;
  let result=(||{
   rx.recv_timeout(Duration::from_secs(10)).map_err(|_|"AMP browser creation timed out")??;
   let window=app.get_webview_window(&label).ok_or("AMP browser unavailable")?;
   // Fetching is denied at the browser protocol level as well as navigation callbacks.
   audit_protocol(&window,"Network.enable",serde_json::json!({}),Duration::from_secs(2))?;
   audit_protocol(&window,"Network.setBlockedURLs",serde_json::json!({"urls":["*"]}),Duration::from_secs(2))?;
   let expression=format!("{}\n;({})({})",include_str!("../../assets/third-party/amp-validator/validator_wasm.js"),include_str!("amp-run.js"),serde_json::to_string(source).map_err(|e|e.to_string())?);
   let raw=audit_protocol(&window,"Runtime.evaluate",serde_json::json!({"expression":expression,"awaitPromise":true,"returnByValue":true,"timeout":10000}),Duration::from_secs(12))?;
   let entry=fitig_engine::javascript::decode("AMP",&raw,0);if !entry.error.is_empty(){return Err(entry.error)}
   fitig_engine::amp::decode(entry.value.ok_or("AMP validator returned no report")?)
  })();if let Some(window)=app.get_webview_window(&label){let _=window.close();}result
 }
}
pub fn render(app:&tauri::AppHandle,url:&str,options:&fitig_engine::render::Rendering,user_agent:&str)->fitig_engine::Result<fitig_engine::render::RenderedDocument>{
 if !options.binary.is_empty(){return fitig_engine::render::render_with_user_agent(url,options,user_agent).map(Into::into)}
 #[cfg(not(windows))]{return fitig_engine::render::render(url,options).map(Into::into)}
 #[cfg(windows)]{
  let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();let label=format!("render-{nonce}");let window_label=label.clone();let app_for_window=app.clone();let target:tauri::Url=url.parse()?;let blank:tauri::Url="about:blank".parse()?;let user_agent=user_agent.to_string();let (created,creation)=mpsc::channel();let (loaded,load)=mpsc::channel();let width=options.viewport_width as f64;let height=options.viewport_height as f64;
  app.run_on_main_thread(move||{
   let result=tauri::WebviewWindowBuilder::new(&app_for_window,&window_label,tauri::WebviewUrl::External(blank)).visible(false).inner_size(width,height).focused(false).skip_taskbar(true).user_agent(&user_agent)
    .data_directory(std::env::temp_dir().join(format!("fitig-render-webview-{}",std::process::id())))
    .on_new_window(|_,_|tauri::webview::NewWindowResponse::Deny)
    .on_download(|_,_|false).on_navigation(|url|matches!(url.scheme(),"http"|"https")||url.as_str()=="about:blank")
    .on_page_load(move|_,event|{if event.event()==tauri::webview::PageLoadEvent::Finished&&matches!(event.url().scheme(),"http"|"https"){let _=loaded.send(());}}).build().map(|_|()).map_err(|e|e.to_string());let _=created.send(result);
  })?;
  let timeout=Duration::from_millis(options.timeout_ms);let start=std::time::Instant::now();
  let diagnostics=std::sync::Arc::new(std::sync::Mutex::new(fitig_engine::browser_logs::Report::default()));
  let operation=(||->fitig_engine::Result<fitig_engine::render::RenderedDocument>{
   creation.recv_timeout(timeout).map_err(|_|"WebView2 renderer creation timed out")?.map_err(|e|format!("WebView2 renderer: {e}"))?;
   let window=app.get_webview_window(&label).ok_or("Renderer window unavailable")?;
   if options.capture_mobile{let remaining=timeout.saturating_sub(start.elapsed());audit_protocol(&window,"Emulation.setDeviceMetricsOverride",serde_json::json!({"width":options.viewport_width,"height":options.viewport_height,"deviceScaleFactor":1,"mobile":true,"screenWidth":options.viewport_width,"screenHeight":options.viewport_height}),remaining)?;audit_protocol(&window,"Emulation.setTouchEmulationEnabled",serde_json::json!({"enabled":true,"maxTouchPoints":1}),timeout.saturating_sub(start.elapsed()))?;}
   if options.capture_console{if let Err(error)=enable_console(&window,diagnostics.clone(),timeout.saturating_sub(start.elapsed())){if let Ok(mut report)=diagnostics.lock(){report.error=error;}}}
   window.navigate(target)?;
   load.recv_timeout(timeout.saturating_sub(start.elapsed())).map_err(|_|"WebView2 navigation timed out")?;
   std::thread::sleep(Duration::from_millis(options.wait_ms));
   let window=app.get_webview_window(&label).ok_or("Renderer window unavailable")?;let (complete,completed)=mpsc::channel();let failure=complete.clone();
   window.with_webview(move|view|{
    let result=(||->std::result::Result<(),String>{unsafe{
     let webview=view.controller().CoreWebView2().map_err(|e|e.to_string())?;
     let handler=webview2_com::ExecuteScriptCompletedHandler::create(Box::new(move|status,result|{let value=status.map_err(|e|e.to_string()).and_then(|_|serde_json::from_str::<serde_json::Value>(&result).map_err(|e|e.to_string()));let _=complete.send(value);Ok(())}));
     let script=webview2_com::CoTaskMemPWSTR::from("({html:document.documentElement.outerHTML,url:location.href})");webview.ExecuteScript(*script.as_ref().as_pcwstr(),&handler).map_err(|e|e.to_string())?;Ok(())
    }})();if let Err(error)=result{let _=failure.send(Err(error));}
   })?;
   let result=completed.recv_timeout(timeout.saturating_sub(start.elapsed())).map_err(|_|"WebView2 DOM extraction timed out")?.map_err(|e|format!("WebView2 DOM: {e}"))?;
   let effective=result["url"].as_str().unwrap_or("");if !effective.starts_with("http://")&&!effective.starts_with("https://"){return Err("WebView2 loaded a browser error page instead of an HTTP(S) document".into())}
   let html=result["html"].as_str().ok_or("WebView2 returned no DOM")?;if html.len()>10*1024*1024{return Err("Rendered DOM exceeds 10 MiB limit".into())}let mut document=fitig_engine::render::RenderedDocument{html:html.into(),url:effective.into(),..Default::default()};
   if options.capture_screenshot{match capture_screenshot(&window,timeout.saturating_sub(start.elapsed())){Ok(png)=>document.screenshot_png=Some(png),Err(error)=>document.screenshot_error=error}}
   if options.capture_mobile{document.mobile=Some(evaluate_mobile(&window,options.viewport_width,options.viewport_height,timeout.saturating_sub(start.elapsed())));}
   if options.capture_accessibility{document.accessibility=Some(evaluate_accessibility(&window,timeout.saturating_sub(start.elapsed())));}
   if !options.custom_javascript.is_empty(){let mut report=fitig_engine::javascript::Report::default();let mut exhausted=false;for rule in &options.custom_javascript{if exhausted||start.elapsed()>=timeout{report.results.push(fitig_engine::javascript::failed(&rule.name,"Renderer time budget exhausted; script was not run",0));continue}let budget=Duration::from_millis(rule.timeout_ms.saturating_add(500)).min(timeout.saturating_sub(start.elapsed()));let(entry,deadline)=evaluate_custom_script(&window,rule,budget);report.results.push(entry);exhausted=deadline;}document.javascript=Some(report);}
   if options.capture_console{document.console=Some(diagnostics.lock().map_err(|_|"Browser diagnostic state unavailable")?.clone());}
   Ok(document)
  })();
  if let Some(window)=app.get_webview_window(&label){let _=window.close();}operation
 }
}

#[cfg(windows)]fn audit_protocol(window:&tauri::WebviewWindow,method:&str,args:serde_json::Value,budget:Duration)->Result<serde_json::Value,String>{
 if budget.is_zero(){return Err("Accessibility host time budget exhausted".into())}
 let method=method.to_string();let args=args.to_string();let(done,result)=mpsc::channel();let failure=done.clone();
 window.with_webview(move|view|{let operation=(||->Result<(),String>{unsafe{
  let webview=view.controller().CoreWebView2().map_err(|error|error.to_string())?;
  let callback=webview2_com::CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move|status,raw|{let value=status.map_err(|error|error.to_string()).and_then(|_|{if raw.len()>128*1024{return Err("Accessibility protocol result exceeds 128 KiB".into())}serde_json::from_str(&raw).map_err(|error|error.to_string())});let _=done.send(value);Ok(())}));
  let name=webview2_com::CoTaskMemPWSTR::from(method.as_str());let args=webview2_com::CoTaskMemPWSTR::from(args.as_str());webview.CallDevToolsProtocolMethod(*name.as_ref().as_pcwstr(),*args.as_ref().as_pcwstr(),&callback).map_err(|error|error.to_string())?;Ok(())
 }})();if let Err(error)=operation{let _=failure.send(Err(error));}}).map_err(|error|error.to_string())?;
 let payload:serde_json::Value=result.recv_timeout(budget).map_err(|_|"Browser audit protocol host deadline exceeded".to_string())??;
 if let Some(error)=payload.get("error"){return Err(format!("Browser audit protocol error: {}",error.get("message").and_then(|value|value.as_str()).unwrap_or("Unknown protocol error")))}Ok(payload)
}
#[cfg(windows)]fn evaluate_accessibility(window:&tauri::WebviewWindow,budget:Duration)->fitig_engine::accessibility::Report{
 let start=std::time::Instant::now();let budget=budget.min(Duration::from_millis(11000));
 let operation=(||->Result<fitig_engine::accessibility::Report,String>{
  let frame=audit_protocol(window,"Page.getFrameTree",serde_json::json!({}),budget.saturating_sub(start.elapsed()))?;
  let frame=frame["frameTree"]["frame"]["id"].as_str().ok_or("Accessibility frame unavailable")?;
  let world=audit_protocol(window,"Page.createIsolatedWorld",serde_json::json!({"frameId":frame,"worldName":"FITIG accessibility audit","grantUniversalAccess":false}),budget.saturating_sub(start.elapsed()))?;
  let context=world["executionContextId"].as_u64().ok_or("Accessibility isolated context unavailable")?;
  let remaining=budget.saturating_sub(start.elapsed());if remaining.is_zero(){return Err("Accessibility host time budget exhausted".into())}
  let raw=audit_protocol(window,"Runtime.evaluate",serde_json::json!({"expression":fitig_engine::accessibility::script(),"contextId":context,"awaitPromise":true,"returnByValue":true,"timeout":remaining.as_millis().min(10000)}),remaining)?;
  let decoded=fitig_engine::javascript::decode("Accessibility",&raw,start.elapsed().as_millis());if !decoded.error.is_empty(){return Err(decoded.error)}
  let report=serde_json::from_value(decoded.value.ok_or("Accessibility returned no result")?).map_err(|error|error.to_string())?;
  fitig_engine::accessibility::validate(&report).map_err(|error|error.to_string())?;Ok(report)
 })();operation.unwrap_or_else(|error|fitig_engine::accessibility::failed(&error))
}
#[cfg(windows)]fn evaluate_mobile(window:&tauri::WebviewWindow,width:u32,height:u32,budget:Duration)->fitig_engine::mobile::Report{
 let start=std::time::Instant::now();let budget=budget.min(Duration::from_millis(5000));
 let operation=(||->Result<fitig_engine::mobile::Report,String>{
  let frame=audit_protocol(window,"Page.getFrameTree",serde_json::json!({}),budget.saturating_sub(start.elapsed()))?;let frame=frame["frameTree"]["frame"]["id"].as_str().ok_or("Mobile frame unavailable")?;
  let world=audit_protocol(window,"Page.createIsolatedWorld",serde_json::json!({"frameId":frame,"worldName":"FITIG mobile checks","grantUniversalAccess":false}),budget.saturating_sub(start.elapsed()))?;let context=world["executionContextId"].as_u64().ok_or("Mobile isolated context unavailable")?;
  let remaining=budget.saturating_sub(start.elapsed());if remaining.is_zero(){return Err("Mobile host time budget exhausted".into())}
  let raw=audit_protocol(window,"Runtime.evaluate",serde_json::json!({"expression":fitig_engine::mobile::script(width,height),"contextId":context,"awaitPromise":true,"returnByValue":true,"timeout":remaining.as_millis()}),remaining)?;
  let decoded=fitig_engine::javascript::decode("Mobile",&raw,start.elapsed().as_millis());if !decoded.error.is_empty(){return Err(decoded.error)}let report=serde_json::from_value(decoded.value.ok_or("Mobile returned no result")?).map_err(|error|error.to_string())?;fitig_engine::mobile::validate(&report).map_err(|error|error.to_string())?;Ok(report)
 })();operation.unwrap_or_else(|error|fitig_engine::mobile::failed(&error))
}
#[cfg(windows)]fn evaluate_custom_script(window:&tauri::WebviewWindow,rule:&fitig_engine::javascript::Rule,budget:Duration)->(fitig_engine::javascript::Entry,bool){
 let start=std::time::Instant::now();let name=rule.name.clone();let params=serde_json::json!({"expression":fitig_engine::javascript::expression(rule),"awaitPromise":true,"returnByValue":true,"timeout":rule.timeout_ms}).to_string();let(done,result)=mpsc::channel();let failure=done.clone();
 let submitted=window.with_webview(move|view|{let operation=(||->std::result::Result<(),String>{unsafe{let webview=view.controller().CoreWebView2().map_err(|error|error.to_string())?;let callback=webview2_com::CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move|status,raw|{let value=status.map_err(|error|error.to_string()).and_then(|_|if raw.len()>1024*1024{Err("Browser script protocol result exceeds 1 MiB".into())}else{serde_json::from_str::<serde_json::Value>(&raw).map_err(|error|error.to_string())});let _=done.send(value);Ok(())}));let method=webview2_com::CoTaskMemPWSTR::from("Runtime.evaluate");let args=webview2_com::CoTaskMemPWSTR::from(params.as_str());webview.CallDevToolsProtocolMethod(*method.as_ref().as_pcwstr(),*args.as_ref().as_pcwstr(),&callback).map_err(|error|error.to_string())?;Ok(())}})();if let Err(error)=operation{let _=failure.send(Err(error));}});
 if let Err(error)=submitted{return(fitig_engine::javascript::failed(&name,&error.to_string(),start.elapsed().as_millis()),false)}
 match result.recv_timeout(budget.saturating_sub(start.elapsed())){Ok(Ok(payload))=>(fitig_engine::javascript::decode(&name,&payload,start.elapsed().as_millis()),false),Ok(Err(error))=>(fitig_engine::javascript::failed(&name,&error,start.elapsed().as_millis()),false),Err(_)=>(fitig_engine::javascript::failed(&name,"Browser script host deadline exceeded",start.elapsed().as_millis()),true)}
}

#[cfg(windows)]fn capture_screenshot(window:&tauri::WebviewWindow,timeout:Duration)->std::result::Result<Vec<u8>,String>{
 let (done,result)=mpsc::channel();let failure=done.clone();
 window.with_webview(move|view|{
  let operation=(||->std::result::Result<(),String>{unsafe{
   let webview=view.controller().CoreWebView2().map_err(|error|error.to_string())?;
   let callback=webview2_com::CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move|status,value|{let payload=status.map_err(|error|error.to_string()).and_then(|_|serde_json::from_str::<serde_json::Value>(&value).map_err(|error|error.to_string())).and_then(|json|{let encoded=json["data"].as_str().ok_or("Browser returned no PNG data")?;fitig_engine::screenshots::decode(encoded).map_err(|error|error.to_string())});let _=done.send(payload);Ok(())}));
   let method=webview2_com::CoTaskMemPWSTR::from("Page.captureScreenshot");let args=webview2_com::CoTaskMemPWSTR::from("{\"format\":\"png\",\"captureBeyondViewport\":false,\"fromSurface\":true}");
   webview.CallDevToolsProtocolMethod(*method.as_ref().as_pcwstr(),*args.as_ref().as_pcwstr(),&callback).map_err(|error|error.to_string())?;Ok(())
  }})();if let Err(error)=operation{let _=failure.send(Err(error));}
 }).map_err(|error|error.to_string())?;
 result.recv_timeout(timeout).map_err(|_|"Browser screenshot capture timed out".to_string())?
}

#[cfg(windows)]fn enable_console(window:&tauri::WebviewWindow,capture:std::sync::Arc<std::sync::Mutex<fitig_engine::browser_logs::Report>>,timeout:Duration)->std::result::Result<(),String>{
 let (ready,enabled)=mpsc::channel();let failed=ready.clone();
 window.with_webview(move|view|{
  let operation=(||->std::result::Result<(),String>{unsafe{
   let webview=view.controller().CoreWebView2().map_err(|error|error.to_string())?;
   for event in ["Runtime.consoleAPICalled","Runtime.exceptionThrown"]{
    let name=webview2_com::CoTaskMemPWSTR::from(event);let receiver=webview.GetDevToolsProtocolEventReceiver(*name.as_ref().as_pcwstr()).map_err(|error|error.to_string())?;let captured=capture.clone();
    let callback=webview2_com::DevToolsProtocolEventReceivedEventHandler::create(Box::new(move|_,args|{if let Some(args)=args{let mut raw=webview2_com::CoTaskMemPWSTR::default().take();let status=args.ParameterObjectAsJson(&mut raw);let value=webview2_com::CoTaskMemPWSTR::from(raw);if let Ok(mut report)=captured.lock(){match status{Ok(())=>report.record_raw(event,&value.to_string()),Err(error)=>report.error=error.to_string()}}}Ok(())}));let mut token=0;receiver.add_DevToolsProtocolEventReceived(&callback,&mut token).map_err(|error|error.to_string())?;
   }
   let callback=webview2_com::CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move|status,_|{let _=ready.send(status.map_err(|error|error.to_string()));Ok(())}));let method=webview2_com::CoTaskMemPWSTR::from("Runtime.enable");let args=webview2_com::CoTaskMemPWSTR::from("{}");webview.CallDevToolsProtocolMethod(*method.as_ref().as_pcwstr(),*args.as_ref().as_pcwstr(),&callback).map_err(|error|error.to_string())?;Ok(())
  }})();if let Err(error)=operation{let _=failed.send(Err(error));}
 }).map_err(|error|error.to_string())?;
 enabled.recv_timeout(timeout).map_err(|_|"Browser console capture setup timed out".to_string())?
}

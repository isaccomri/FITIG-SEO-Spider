const fs=require('fs'),path=require('path'),{spawn}=require('child_process');
const {chromium}=require(process.env.FITIG_PLAYWRIGHT_MODULE||'playwright');
async function withNativeApp(action){
 const executable=path.resolve(process.env.FITIG_DESKTOP_EXECUTABLE||path.resolve(__dirname,'../../src-tauri/target/debug/fitig-seo-spider.exe'));
 if(!fs.existsSync(executable))throw Error('Build the desktop executable or set FITIG_DESKTOP_EXECUTABLE');
 const output=path.resolve(process.env.FITIG_ACCEPTANCE_DIR||path.resolve(__dirname,'../../acceptance/native-text-analysis'));fs.mkdirSync(output,{recursive:true});
 const work=path.resolve(process.env.FITIG_TEST_WORK_DIR||path.join(process.cwd(),'work'));fs.mkdirSync(work,{recursive:true});
 const port=Number(process.env.FITIG_CDP_PORT||9271);let spawnError,browser,page;
 const app=spawn(executable,[],{cwd:path.dirname(executable),windowsHide:false,stdio:'inherit',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port='+port,WEBVIEW2_USER_DATA_FOLDER:path.join(work,'text-analysis-profile-'+Date.now())}});
 app.on('error',error=>spawnError=error);
 try{
  let endpoint;for(let attempt=0;attempt<60;attempt++){
   if(spawnError)throw spawnError;if(app.exitCode!==null)throw Error('Owned app exited: '+app.exitCode);
   try{endpoint=(await(await fetch('http://127.0.0.1:'+port+'/json/version',{signal:AbortSignal.timeout(1000)})).json()).webSocketDebuggerUrl}catch{}
   if(endpoint)break;await new Promise(resolve=>setTimeout(resolve,500));
  }
  if(!endpoint)throw Error('Owned WebView2 debugging endpoint unavailable');
  browser=await chromium.connectOverCDP(endpoint);page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(30000);
  await page.getByText('Engine connected',{exact:true}).waitFor();
  const invoke=(command,args)=>page.evaluate(([command,args])=>window.__TAURI_INTERNALS__.invoke(command,args),[command,args]);
  await action({page,invoke,output,executable,processId:app.pid});
 }catch(error){if(page){await page.screenshot({path:path.join(output,'failure.png'),fullPage:true}).catch(()=>{});const text=await page.locator('body').innerText().catch(()=>'');fs.writeFileSync(path.join(output,'failure.txt'),error.stack+'\n\n'+text)}throw error}
 finally{if(browser)await browser.close().catch(()=>{});if(app.exitCode===null)app.kill()}
}
module.exports={withNativeApp};

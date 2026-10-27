// Reuse an owned Windows WebView2 instance for browser regressions; no Chrome flags or policies change.
const fs=require('fs'),path=require('path'),http=require('http'),{spawn}=require('child_process');
const {chromium}=require('C:/Users/isacc/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const root=process.cwd(),folder=path.join(root,'outputs/FITIG-SEO-Spider-0.1.6-Windows'),out=path.join(root,'outputs/FITIG-SEO-Spider/acceptance/regression-016');fs.mkdirSync(out,{recursive:true});
const server=http.createServer((req,res)=>{res.setHeader('Content-Type','text/html; charset=utf-8');res.end('<!doctype html><title>Independent synthetic test fixture</title>')});
function run(file,env){return new Promise((resolve,reject)=>{const child=spawn(process.execPath,[file],{cwd:root,env:{...process.env,...env},stdio:'inherit'});child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(file+' exit '+code)))})}
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r));const app=spawn(path.join(folder,'FITIG-SEO-Spider.exe'),[],{cwd:folder,stdio:'inherit',windowsHide:false,env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9261',WEBVIEW2_USER_DATA_FOLDER:path.join(root,'work/regression-'+Date.now())}});let browser;
try{let endpoint;for(let i=0;i<50;i++){try{endpoint=(await(await fetch('http://127.0.0.1:9261/json/version')).json()).webSocketDebuggerUrl;if(endpoint)break}catch{}await new Promise(r=>setTimeout(r,500))}if(!endpoint)throw Error('No native browser');browser=await chromium.connectOverCDP(endpoint);await browser.contexts()[0].pages()[0].getByText('Engine connected',{exact:true}).waitFor();await browser.contexts()[0].pages()[0].goto('http://127.0.0.1:'+server.address().port+'/');
 await browser.contexts()[0].pages()[0].evaluate(()=>{delete window.__TAURI_INTERNALS__;delete window.__TAURI__;window.isTauri=false;});
 await run('outputs/FITIG-SEO-Spider/ui/tests/visualisation.browser.cjs',{FITIG_PLAYWRIGHT_MODULE:'C:/Users/isacc/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright',FITIG_BROWSER_CDP:endpoint});
}finally{if(browser)await browser.close().catch(()=>{});if(app.exitCode===null)app.kill();await new Promise(r=>server.close(r))}
fs.copyFileSync('outputs/FITIG-SEO-Spider/acceptance/yaelmor/crawl-content-area.sqlite',path.join(out,'crawl-content-area.sqlite'));
const env={FITIG_PLAYWRIGHT_MODULE:'C:/Users/isacc/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright',FITIG_DESKTOP_EXECUTABLE:path.join(folder,'FITIG-SEO-Spider.exe'),FITIG_ACCEPTANCE_DIR:out,FITIG_CDP_PORT:'9263'};
await run('outputs/FITIG-SEO-Spider/ui/tests/desktop-acceptance.cjs',env);await run('outputs/FITIG-SEO-Spider/ui/tests/desktop-workflows.cjs',env);
})().catch(e=>{console.error(e.stack);process.exitCode=1;server.close()});

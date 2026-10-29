const fs=require('fs'),path=require('path'),http=require('http');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[];function check(ok,label){if(!ok)throw Error(label);checks.push(label);console.log('PASS '+label)}
const server=http.createServer((req,res)=>{res.setHeader('Connection','close');res.setHeader('Content-Type','text/html; charset=utf-8');res.end(req.url==='/robots.txt'?'User-agent: *\nAllow: /':req.url==='/'?`<base href='/he/'><base target='חלון'><base target='_parent'><title>בית</title><a href='one'>א</a><a href='two' target='_self'>ב</a><a href='three' target=''>ג</a><area href='four'><script src='app.js'></script>`:'<title>עמוד</title>')});
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;try{await withNativeApp(async({page,invoke,output})=>{
 const db=path.join(output,'base-target.sqlite');await invoke('start_crawl',{url:origin+'/',path:db,mode:'spider',urls:null,limit:0,delayMs:0,resume:false,profile:{}});
 let s;for(let i=0;i<400;i++){s=await invoke('get_snapshot');if(!s.running)break;await page.waitForTimeout(50)}
 check(!s.running&&!s.error,'Native crawl completes');const home=s.pages.find(p=>p.url===origin+'/');
 check(s.pages.length===6,'Six independently enumerated fixture URLs crawled');
 check(JSON.stringify(home.links.map(l=>l.target))===JSON.stringify(['חלון','_self','','חלון','']),'Native engine records inherited Hebrew target, explicit overrides and resource exclusion');
 check(home.links[0].url===origin+'/he/one','Independent href base remains effective');
 const before=fs.readFileSync(db);await invoke('load_project',{path:db});s=await invoke('get_snapshot');
 check(s.pages.find(p=>p.url===origin+'/').links[0].target==='חלון','Inherited target persists across project reload');
 check(fs.readFileSync(db).equals(before),'Read-only reload preserves database bytes');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,version:'0.1.50',checks},null,2));
 })}finally{await new Promise(r=>server.close(r))}})().catch(e=>{console.error(e);process.exitCode=1});

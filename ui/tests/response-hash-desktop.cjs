const fs=require('fs'),path=require('path'),http=require('http'),crypto=require('crypto');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[],traffic=[],arrivals=[];let variant=false;
const raw='<body><main>שלום עולם</main><!-- original --></body>';
const digest=b=>crypto.createHash('sha256').update(b).digest('hex');
const server=http.createServer((req,res)=>{traffic.push(req.url);if(req.url!=='/robots.txt')arrivals.push(process.hrtime.bigint());res.setHeader('Connection','close');if(req.url==='/robots.txt')return res.end('User-agent: *\nAllow: /');res.setHeader('Content-Type','text/html; charset=utf-8');if(req.url==='/')return res.end('<a href="/a">a</a><a href="/b">b</a><a href="/c">c</a><a href="/empty">empty</a><a href="/large">large</a>');if(req.url==='/empty'){res.statusCode=204;return res.end()}if(req.url==='/large')return res.end(Buffer.alloc(11*1024*1024,65));return res.end(req.url==='/c'||variant&&req.url==='/a'?raw.replace('original','changed'):raw)});
function check(value,message){if(!value)throw Error(message);checks.push(message);console.log('PASS '+message)}
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;try{await withNativeApp(async({page,invoke,output})=>{
 async function crawl(name,delayMs=0){const db=path.join(output,name+'.sqlite');await invoke('start_crawl',{url:origin+'/',path:db,mode:'spider',urls:null,limit:0,delayMs,resume:false,profile:{}});let s;for(let n=0;n<600;n++){s=await invoke('get_snapshot');if(!s.running)break;await page.waitForTimeout(50)}if(s.running||s.error)throw Error(s.error||'Crawl timeout');return{db,pages:s.pages}}
 const old=await crawl('baseline'),by=Object.fromEntries(old.pages.map(p=>[new URL(p.url).pathname,p]));
 check(old.pages.length===6,'Independently enumerated six owned URLs fetched');
 check(by['/a'].response_hash===digest(Buffer.from(raw)),'Recorded SHA matches independent Node digest of UTF-8 payload');
 check(by['/a'].response_hash===by['/b'].response_hash,'Identical response bytes share their SHA');
 check(by['/a'].response_hash!==by['/c'].response_hash&&by['/a'].content_hash===by['/c'].content_hash,'Different comments change byte hash while normalized content stays equal');
 check(by['/empty'].response_hash===digest(Buffer.alloc(0)),'Known empty HTTP response has the standard empty digest');
 check(by['/large'].response_hash===null,'Truncated oversized payload remains unknown');
 await page.keyboard.press('F5');await page.waitForTimeout(200);await page.getByRole('tablist',{name:'Result categories'}).getByRole('tab',{name:'Content',exact:true}).click();await page.locator('.filterbar select').selectOption('Duplicate response body');await page.waitForTimeout(100);
 const rows=page.locator('.grid-wrap tbody tr.data-row');check(await rows.count()===2,'Actual exact-body duplicate filter includes only the two byte-identical pages');
 check((await rows.first().locator('td').allTextContents()).includes(digest(Buffer.from(raw))),'Actual Content result cell displays the exact SHA');
 const requests=traffic.length,before=fs.readFileSync(old.db),analysis=await invoke('analyze_crawl');check(Object.keys(analysis.duplicate_response_bodies).length===1&&analysis.duplicate_response_bodies[digest(Buffer.from(raw))].length===2,'Read-only analysis groups only observed matching payloads');
 await invoke('load_project',{path:old.db});check(fs.readFileSync(old.db).equals(before)&&traffic.length===requests,'Reload and analysis preserve database bytes and issue no HTTP');
 variant=true;const current=await crawl('current'),comparison=await invoke('compare_crawls',{path:old.db});const row=comparison.rows.find(r=>r.url===origin+'/a');check(row.fields.includes('response_hash')&&!row.fields.includes('content_hash'),'Actual compare command reports byte-only change independently of normalized content');
 const start=arrivals.length;await crawl('paced',80);const paced=arrivals.slice(start);check(paced.length===6&&paced.slice(1).every((time,i)=>Number(time-paced[i])/1e6>=60),'Actual Desktop crawl applies configured global request pacing after robots resolution');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,baseline:old.db,current:current.db,version:'0.1.46',scope:'Owned local HTTP, actual Tauri commands and Content table; no competitor/account equivalence claim'},null,2));await page.screenshot({path:path.join(output,'content-hash.png'),fullPage:true});
 })}finally{await new Promise(r=>server.close(r))}})().catch(e=>{console.error(e);process.exitCode=1});

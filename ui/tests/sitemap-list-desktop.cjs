const fs=require('fs'),path=require('path'),http=require('http');const {withNativeApp}=require('./native-driver.cjs');
const checks=[],traffic=[];function check(ok,label){if(!ok)throw Error(label);checks.push(label);console.log('PASS '+label)}
const server=http.createServer((req,res)=>{traffic.push(req.url);res.setHeader('Connection','close');res.setHeader('Content-Type','text/html; charset=utf-8');res.end(req.url==='/robots.txt'?'User-agent: *\nDisallow: /blocked':`<title>שלום</title><a href='/discovered'>extra</a>`)});
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;try{await withNativeApp(async({page,invoke,output})=>{
 const db=path.join(output,'source.sqlite');await invoke('start_crawl',{url:origin+'/seed',path:db,mode:'list',urls:[origin+'/seed'],limit:0,delayMs:0,resume:false,profile:{}});
 async function complete(){let s;for(let i=0;i<400;i++){s=await invoke('get_snapshot');if(!s.running)break;await page.waitForTimeout(50)}if(s.running||s.error)throw Error(s.error||'Timeout');return s}await complete();
 const urls=[origin+'/עברית',origin+'/%D7%A2%D7%91%D7%A8%D7%99%D7%AA',origin+'/blocked'];await invoke('save_sitemap_inventory',{inventory:{locations:urls,files:['owned.xml'],missing:[]}});await page.keyboard.press('F5');await page.waitForTimeout(100);
 const before=fs.readFileSync(db),requests=traffic.length;await page.getByRole('button',{name:'Sitemaps',exact:true}).click();await page.getByRole('button',{name:'Validate XML sitemap…',exact:true}).click();
 await page.getByRole('button',{name:'Prepare sitemap URL crawl…',exact:true}).click();const list=page.getByRole('dialog',{name:'list',exact:true}).locator('textarea');check((await list.inputValue()).split('\n').length===2,'Review contains two canonical deduplicated URLs');
 check(traffic.length===requests&&fs.readFileSync(db).equals(before),'Preparing List review sends no HTTP and preserves source database');
 await page.getByRole('button',{name:'Use List mode',exact:true}).click();check(traffic.length===requests,'Accepting configuration does not start a crawl');
 await page.getByRole('button',{name:/Start$/}).click();await page.waitForTimeout(100);const result=await complete();
 check(result.project_path!==db&&fs.readFileSync(db).equals(before),'Start creates another project and preserves source crawl');
 const loaded=await invoke('load_project',{path:result.project_path});check(loaded.mode==='list'&&loaded.list_urls.length===2,'New crawl persists its reviewed List inventory');
 check(traffic.some(url=>url.startsWith('/%D7'))&&!traffic.includes('/blocked')&&!traffic.includes('/discovered'),'Hebrew target fetched; robots-disallowed and discovered links not fetched');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,project:result.project_path},null,2));
 })}finally{await new Promise(r=>server.close(r))}})().catch(e=>{console.error(e);process.exitCode=1});

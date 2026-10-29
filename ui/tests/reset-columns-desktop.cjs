const fs=require('fs'),path=require('path'),http=require('http');const{withNativeApp}=require('./native-driver.cjs');const checks=[],traffic=[];function check(ok,label){if(!ok)throw Error(label);checks.push(label);console.log('PASS '+label)}
const server=http.createServer((req,res)=>{traffic.push(req.url);res.setHeader('Connection','close');res.setHeader('Content-Type','text/html');res.end(req.url==='/robots.txt'?'User-agent: *\nAllow: /':'<title>Fixture</title>')});
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin='http://127.0.0.1:'+server.address().port;try{await withNativeApp(async({page,invoke,output})=>{
 const db=path.join(output,'source.sqlite');await invoke('start_crawl',{url:origin+'/',path:db,mode:'spider',urls:null,limit:0,delayMs:0,resume:false,profile:{}});for(let i=0;i<100;i++){if(!(await invoke('get_snapshot')).running)break;await page.waitForTimeout(50)}
 const settings={version:1,dark:false,hebrew:false,details_ratio:.5,overview_width:300,columns:{Internal:['title','url'],'Meta Keywords':['meta_keywords_first','url']},widths:{Internal:{url:900},'Meta Keywords':{url:800}}};
 await page.evaluate(p=>localStorage.setItem('fitig.workbench.v1',JSON.stringify(p)),settings);await page.reload();await page.getByText('Engine connected',{exact:true}).waitFor();await page.waitForTimeout(250);
 check((await page.locator('.grid-wrap thead th').allTextContents())[1].includes('Title'),'Seeded nondefault order appears in actual table');
 const before=fs.readFileSync(db),requests=traffic.length;await page.getByRole('button',{name:'View',exact:true}).click();await page.getByRole('button',{name:'Reset Columns for All Tables',exact:true}).click();await page.waitForTimeout(350);
 check((await page.locator('.grid-wrap thead th').allTextContents())[1].includes('Address'),'View action restores default visible column order');
 const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('fitig.workbench.v1')));
 check(Object.keys(saved.columns).length===0&&Object.keys(saved.widths).length===0,'All tab orders and widths are reset in persisted preferences');
 check(saved.dark===false&&saved.hebrew===false&&saved.details_ratio===.5&&saved.overview_width===300,'Theme language and pane geometry remain selected');
 await page.getByRole('tablist',{name:'Result categories'}).getByRole('tab',{name:'Meta Keywords',exact:true}).click();check((await page.locator('.grid-wrap thead th').allTextContents())[1].includes('Address'),'Inactive tab also receives default order');
 check(fs.readFileSync(db).equals(before)&&traffic.length===requests,'Reset and tab switch preserve database bytes without network');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks},null,2));
 })}finally{await new Promise(r=>server.close(r))}})().catch(e=>{console.error(e);process.exitCode=1});

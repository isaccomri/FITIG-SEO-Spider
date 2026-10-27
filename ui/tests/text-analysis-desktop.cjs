const fs=require('fs'),path=require('path'),http=require('http');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[],requests=[];
function check(condition,message){if(!condition)throw Error(message);checks.push(message);console.log('PASS: '+message)}
const server=http.createServer((request,response)=>{
 requests.push(request.url);response.setHeader('Connection','close');response.setHeader('Content-Type','text/html; charset=utf-8');
 if(request.url==='/robots.txt'){response.setHeader('Content-Type','text/plain');return response.end('User-agent: *\nAllow: /')}
 const bodies={a:'<nav>navigation alpha</nav><main>שלום עולם שלום עולם חדש</main><textarea>private control</textarea>',b:'<nav>navigation beta</nav><main>שלום עולם שלום עולם חדש</main>',c:'completely different unrelated content sample',short:'קצר'};
 const name=request.url.slice(1);if(!bodies[name])response.statusCode=404;response.end('<!doctype html><meta charset="utf-8"><title>'+name+'</title><body>'+(bodies[name]||'missing')+'</body>');
});
(async()=>{await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin='http://127.0.0.1:'+server.address().port;
try{await withNativeApp(async({page,invoke,output,executable})=>{
 const db=path.join(output,'source-'+Date.now()+'.sqlite');
 async function crawl(project,store,urls){await invoke('start_crawl',{url:urls[0],limit:0,delayMs:0,path:project,resume:false,mode:'list',urls,profile:{store_html:store,rendering:{enabled:false,binary:'',wait_ms:500,timeout_ms:10000}}});let snapshot;for(let attempt=0;attempt<100;attempt++){snapshot=await invoke('get_snapshot');if(!snapshot.running)break;await new Promise(resolve=>setTimeout(resolve,200))}if(snapshot.running||snapshot.error)throw Error(snapshot.error||'Fixture crawl timed out');return snapshot}
 const snapshot=await crawl(db,true,['a','b','c','short'].map(name=>origin+'/'+name));check(snapshot.pages.length===4,'Actual native List crawl stores four HTML fixture sources');
 const baselineRequests=requests.length,options={n:2,limit:1000,min_frequency:2,min_documents:1,scope:'content',max_pages:500,include_pattern:'',stop_words:[],threshold:.9,shingle_size:5};
 const grams=await invoke('analyze_saved_text',{kind:'ngrams',options});
 check(grams.summary.processed_pages===4&&grams.rows[0].phrase==='שלום עולם'&&grams.rows[0].occurrences===4&&grams.rows[0].documents===2,'Native Rust N-grams retain Hebrew occurrence and document counts');
 check(!grams.rows.some(row=>/private|navigation/.test(row.phrase)),'Content analysis excludes navigation and form-value containers');
 const duplicates=await invoke('analyze_saved_text',{kind:'duplicates',options});
 check(duplicates.rows.length===1&&duplicates.rows[0].similarity===1,'Native Rust duplicate report finds the identical content pair');
 check(duplicates.summary.insufficient_text.some(url=>url.endsWith('/short')),'Short documents are reported as insufficient for configured shingles');
 async function menu(action){await page.getByRole('button',{name:'Crawl Analysis',exact:true}).click();await page.getByRole('button',{name:action,exact:true}).click()}
 await menu('N-gram analysis…');await page.getByText('4 pages analyzed · 3 matching phrases',{exact:true}).waitFor();
 check(await page.locator('[aria-label="N-gram analysis"] tbody tr').count()===3,'Actual N-gram report UI displays three calculated phrases');
 const csv=path.join(output,'hebrew-ngrams.csv');await page.getByLabel('Text analysis CSV path',{exact:true}).fill(csv);await page.getByRole('button',{name:'Export text analysis',exact:true}).click();await page.getByText('Saved: '+csv,{exact:true}).waitFor();check(fs.readFileSync(csv,'utf8').includes('שלום עולם,4,2'),'Actual N-gram CSV exports Hebrew and measured counts');
 await page.getByLabel('N-gram size',{exact:true}).fill('1');await page.getByLabel('Text area',{exact:true}).selectOption('body');await page.getByRole('button',{name:'Analyze saved text',exact:true}).click();await page.getByText('navigation',{exact:true}).waitFor();check(true,'Body scope deliberately includes the navigation vocabulary');
 await page.getByLabel('Stop words (one per line)',{exact:true}).fill('navigation\nעולם');await page.getByRole('button',{name:'Analyze saved text',exact:true}).click();await page.getByText('4 pages analyzed · 2 matching phrases',{exact:true}).waitFor();check(await page.getByText('navigation',{exact:true}).count()===0,'UI stop-word configuration removes the selected terms');
 await page.getByLabel('Maximum analyzed pages',{exact:true}).fill('1');await page.getByRole('button',{name:'Analyze saved text',exact:true}).click();await page.getByRole('alert').filter({hasText:'eligible HTML pages exceed'}).waitFor();check(await page.locator('[aria-label="N-gram analysis"] tbody').count()===0,'Analysis limit error does not display stale successful results');
 await page.getByRole('dialog').getByRole('button',{name:'Close',exact:true}).click();
 await menu('Near-duplicate content…');await page.getByText('4 pages analyzed · 1 matching pairs',{exact:true}).waitFor();
 check(await page.locator('[aria-label="Near-duplicate analysis"] tbody tr').count()===1,'Actual near-duplicate UI displays the calculated pair');
 const pairCsv=path.join(output,'duplicate-pairs.csv');await page.getByLabel('Text analysis CSV path',{exact:true}).fill(pairCsv);await page.getByRole('button',{name:'Export text analysis',exact:true}).click();await page.getByText('Saved: '+pairCsv,{exact:true}).waitFor();check(fs.readFileSync(pairCsv,'utf8').includes('100.0000'),'Native near-duplicate export retains the measured similarity');
 await page.screenshot({path:path.join(output,'near-duplicate-report.png'),fullPage:true});await page.getByRole('dialog').getByRole('button',{name:'Close',exact:true}).click();
 check(requests.length===baselineRequests,'N-gram and duplicate analysis make no additional fixture network requests');
 const missingDb=path.join(output,'without-source-'+Date.now()+'.sqlite');await crawl(missingDb,false,[origin+'/a']);await menu('N-gram analysis…');await page.getByRole('alert').filter({hasText:'No stored successful HTML sources'}).waitFor();check(await page.locator('[aria-label="N-gram analysis"] tbody').count()===0,'Missing saved HTML produces a real error rather than demonstration data');
 await page.getByRole('dialog').getByRole('button',{name:'Close',exact:true}).click();await invoke('load_project',{path:db});
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,executable,db,missingDb,checks,requests,grams,duplicates},null,2));
})}finally{await new Promise(resolve=>server.close(resolve))}})().catch(error=>{console.error(error.stack);server.close();process.exitCode=1});

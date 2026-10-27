const fs=require('fs'),path=require('path');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[];function check(condition,message){if(!condition)throw Error(message);checks.push(message);console.log('PASS: '+message)}
withNativeApp(async({page,invoke,output,executable})=>{
 const db=process.env.FITIG_REAL_CRAWL_DB;if(!db||!fs.existsSync(db))throw Error('Set FITIG_REAL_CRAWL_DB to the saved yaelmor acceptance crawl');
 const before=fs.readFileSync(db);const snapshot=await invoke('load_project',{path:db});
 check(snapshot.pages.length===191,'Real saved yaelmor crawl contains the matched 191-URL inventory');
 const options={n:2,limit:1000,min_frequency:2,min_documents:1,scope:'content',max_pages:0,include_pattern:'',stop_words:[],threshold:.9,shingle_size:5};
 let start=Date.now();const ngramPromise=invoke('analyze_saved_text',{kind:'ngrams',options});const pingStart=Date.now();const responsive=await invoke('get_snapshot');const snapshotMs=Date.now()-pingStart;const grams=await ngramPromise;const ngramsMs=Date.now()-start;
 check(responsive.pages.length===191,'Workbench snapshot remains available during background text analysis');
 check(grams.summary.processed_pages===72&&grams.summary.missing_sources.length===0,'Real N-grams analyze all 72 successful stored HTML sources with no page cap');
 check(grams.rows.length>100&&grams.rows.some(row=>/[א-ת]/.test(row.phrase)),'Real report contains measured Hebrew phrases rather than demonstration rows');
 start=Date.now();const duplicates=await invoke('analyze_saved_text',{kind:'duplicates',options});const duplicatesMs=Date.now()-start;
 check(duplicates.summary.processed_pages===72,'Near-duplicate analysis reads the same complete successful HTML corpus');
 check(duplicates.rows.every(row=>row.similarity>=.9&&row.similarity<=1&&row.shared_shingles<=row.union_shingles),'Real duplicate pairs retain valid measured shingle similarity');
 await page.getByRole('button',{name:'Crawl Analysis',exact:true}).click();await page.getByRole('button',{name:'N-gram analysis…',exact:true}).click();await page.getByText(/^72 pages analyzed ·/).waitFor();
 await page.screenshot({path:path.join(output,'real-hebrew-ngrams.png'),fullPage:true});check(true,'Real-crawl native Hebrew N-gram table rendered successfully');
 check(fs.readFileSync(db).equals(before),'Read-only text analysis leaves the saved crawl database bytes unchanged');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,executable,db,checks,timing:{ngramsMs,duplicatesMs,snapshotMs},grams,duplicates},null,2));
}).catch(error=>{console.error(error.stack);process.exitCode=1});

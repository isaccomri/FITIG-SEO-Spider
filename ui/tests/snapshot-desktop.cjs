const fs=require('fs'),path=require('path'),{spawnSync}=require('child_process');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[];function check(ok,text){if(!ok)throw Error(text);checks.push(text);console.log('PASS: '+text)}
withNativeApp(async({page,invoke,output,executable})=>{
 const source=process.env.FITIG_REAL_CRAWL_DB;if(!source)throw Error('Set FITIG_REAL_CRAWL_DB');
 const db=path.join(output,'snapshot-copy-'+Date.now()+'.sqlite');
 const python=process.env.FITIG_PYTHON||'python';
 function sql(script){const r=spawnSync(python,['-X','utf8','-c',script,db,source],{encoding:'utf8'});if(r.status!==0)throw Error(r.stderr)}
 sql('import sqlite3,sys\ns=sqlite3.connect("file:"+sys.argv[2]+"?mode=ro",uri=True);d=sqlite3.connect(sys.argv[1]);s.backup(d);d.close();s.close()');
 const loaded=await invoke('load_project',{path:db});check(loaded.pages.length===191,'Owned saved-crawl copy loads all 191 observations');
 let first=await invoke('get_snapshot');const revision=first.revision;check(typeof revision==='number'&&first.pages.length===191,'Full snapshot includes a revision and complete observations');
 const timings=[];let cached;for(let i=0;i<8;i++){const start=performance.now();cached=await invoke('get_snapshot',{revision});timings.push(performance.now()-start)}
 check(cached.unchanged===true&&!Object.hasOwn(cached,'pages')&&cached.completed===191,'Unchanged native snapshot omits repeated page payload and retains count');
 check(JSON.stringify(cached).length<2000,'Idle snapshot payload stays below 2 kB on the 191-URL corpus');
 const forced=await invoke('get_snapshot',{revision:null});check(forced.pages.length===191,'Explicit refresh returns full observations even at the same revision');
 sql('import sqlite3,sys,json\nc=sqlite3.connect(sys.argv[1]);u,j=c.execute("SELECT url,json FROM pages LIMIT 1").fetchone();p=json.loads(j);p["title"]="External revision Hebrew שלום";c.execute("UPDATE pages SET json=? WHERE url=?",(json.dumps(p),u));c.commit();c.close()');
 const changed=await invoke('get_snapshot',{revision});check(changed.unchanged===false&&changed.revision>revision&&changed.pages.some(p=>p.title==='External revision Hebrew שלום'),'External database write invalidates the native revision and reloads changed fields');
 await page.waitForTimeout(1800);check((await page.locator('body').innerText()).includes('191'),'UI keeps its existing rows/count through idle revision polls');
 await page.keyboard.press('F5');await page.waitForTimeout(700);check(await page.getByText('Engine connected',{exact:true}).isVisible(),'Manual F5 refresh preserves a connected workbench');
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,executable,cachedBytes:JSON.stringify(cached).length,cachedSnapshotMs:timings},null,2));
}).catch(e=>{console.error(e);process.exitCode=1});

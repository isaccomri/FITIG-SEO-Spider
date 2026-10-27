const fs=require('fs'),path=require('path');const {withNativeApp}=require('./native-driver.cjs');
const checks=[];function check(ok,message){if(!ok)throw Error(message);checks.push(message);console.log('PASS: '+message)}
withNativeApp(async({page,invoke,output,executable})=>{
 const db=process.env.FITIG_REAL_CRAWL_DB;if(!db)throw Error('Set FITIG_REAL_CRAWL_DB');const before=fs.readFileSync(db);const snapshot=await invoke('load_project',{path:db});check(snapshot.pages.length===191,'Real saved yaelmor inventory opens read-only for robots discovery');
 await page.getByRole('button',{name:'Sitemaps',exact:true}).click();await page.getByRole('button',{name:'Validate XML sitemap…',exact:true}).click();const section=page.getByRole('region',{name:'Sitemap validation'});await section.getByRole('button',{name:'Discover from robots.txt',exact:true}).click();
 await section.getByRole('alert').filter({hasText:'Robots endpoint returned HTML'}).waitFor();check(true,'Actual live yaelmor HTML challenge is reported explicitly instead of a false empty robots result');
 check(await section.getByRole('table',{name:'Discovered sitemaps'}).count()===0,'Challenge response does not fabricate sitemap declarations');
 check(fs.readFileSync(db).equals(before),'Live robots discovery leaves the saved real crawl bytes unchanged');await page.screenshot({path:path.join(output,'yaelmor-robots-challenge.png'),fullPage:true});fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,executable,db,observed:'HTTP HTML challenge; sitemap discovery unavailable for this live response'},null,2));
}).catch(error=>{console.error(error);process.exitCode=1});

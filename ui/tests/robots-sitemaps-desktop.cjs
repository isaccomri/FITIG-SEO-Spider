const fs=require('fs'),path=require('path'),http=require('http'),zlib=require('zlib');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[],requests=[];let mode='valid';
function check(ok,message){if(!ok)throw Error(message);checks.push(message);console.log('PASS: '+message)}
const ns='http://www.sitemaps.org/schemas/sitemap/0.9';
const server=http.createServer((req,res)=>{
 requests.push(req.url);res.setHeader('Connection','close');res.setHeader('Content-Type','text/plain; charset=utf-8');
 const origin='http://127.0.0.1:'+server.address().port;
 const index=child=>`<sitemapindex xmlns="${ns}"><sitemap><loc>${origin}${child}</loc></sitemap></sitemapindex>`;
 const urls=paths=>`<urlset xmlns="${ns}">${paths.map(p=>`<url><loc>${origin}${p}</loc></url>`).join('')}</urlset>`;
 if(req.url==='/robots.txt'){
  if(mode==='html'){res.setHeader('Content-Type','text/html');return res.end('<!doctype html><html><title>One moment, please...</title>')}
  if(mode==='404'){res.statusCode=404;return res.end('not found')}
  if(mode==='redirect'){res.statusCode=302;res.setHeader('Location','http://localhost:'+server.address().port+'/unexpected-robots');return res.end()}
  const text=mode==='empty'?'User-agent: *\nAllow: /':mode==='bad'?`Sitemap: ${origin}/a.xml\nSitemap: ${origin}/missing.xml`:mode==='cycle'?`Sitemap: ${origin}/cycle-a.xml\nSitemap: ${origin}/cycle-b.xml`:mode==='oversized'?'x'.repeat(1024*1024+1):`\ufeffUser-agent: ignored\nDisallow: /\nSITEMAP: ${origin}/a.xml\nUser-agent: *\nAllow: /\nSitemap: ${origin}/a.xml # duplicate\nSitemap: ${origin}/b.xml\nSitemap: http://localhost:${server.address().port}/external.xml\nSitemap: /invalid-relative.xml`;
  if(mode==='gzip'||mode==='oversized'){res.setHeader('Content-Encoding','gzip');return res.end(zlib.gzipSync(Buffer.from(text)))}return res.end(text);
 }
 res.setHeader('Content-Type','application/xml; charset=utf-8');
 if(req.url==='/a.xml')return res.end(index('/shared.xml'));
 if(req.url==='/b.xml')return res.end(urls(['/common','/extra']));
 if(req.url==='/shared.xml')return res.end(urls(['/שלום','/common']));
 if(req.url==='/cycle-a.xml')return res.end(index('/cycle-b.xml'));
 if(req.url==='/cycle-b.xml')return res.end(index('/cycle-a.xml'));
 if(req.url.startsWith('/seed/')){res.setHeader('Content-Type','text/html; charset=utf-8');return res.end('<title>Owned sitemap seed</title><h1>שלום</h1>')}
 res.statusCode=404;res.end('missing');
});
(async()=>{await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin='http://127.0.0.1:'+server.address().port;
 try{await withNativeApp(async({page,invoke,output,executable})=>{
  const db=path.join(output,'robots-'+Date.now()+'.sqlite');await invoke('start_crawl',{url:origin+'/seed/sub?value=1',path:db,limit:0,delayMs:0,resume:false,mode:'list',urls:[origin+'/seed/sub?value=1'],profile:{respect_robots:false}});
  let snapshot;for(let n=0;n<100;n++){snapshot=await invoke('get_snapshot');if(!snapshot.running){if(snapshot.error)throw Error(snapshot.error);break}await page.waitForTimeout(100)}check(snapshot.pages.length===1,'Owned seed crawl completes before robots discovery');
  await page.keyboard.press('F5');await page.getByRole('button',{name:'Sitemaps',exact:true}).click();await page.getByRole('button',{name:'Validate XML sitemap…',exact:true}).click();
  const section=page.getByRole('region',{name:'Sitemap validation'}),discover=section.getByRole('button',{name:'Discover from robots.txt',exact:true});
  const before=fs.readFileSync(db),requestBase=requests.length;await discover.click();await section.getByText(/3 discovered sitemap URLs/).waitFor();
  check(requests.slice(requestBase).join(',')==='/robots.txt','Discovery reads only root robots.txt, stripping the project seed path and query');
  check(fs.readFileSync(db).equals(before)&&await invoke('get_sitemap_inventory')===null,'Discovery neither changes the crawl bytes nor creates sitemap membership');
  const table=section.getByRole('table',{name:'Discovered sitemaps'});check(await table.locator('tbody tr').count()===3,'Global case-insensitive directives deduplicate while retaining cross-origin declarations');
  check(await table.getByRole('checkbox').nth(2).isDisabled()&&await table.getByRole('checkbox').nth(0).isChecked(),'Same-origin maps are selected and the other origin is displayed separately');
  check(await section.getByText(/Invalid absolute sitemap URL on robots.txt line/).count()===1,'Malformed relative declaration has a visible line warning');
  await section.getByRole('button',{name:'Retrieve selected sitemaps (2)',exact:true}).click();await section.getByText(/3 entries ·/).waitFor();
  let inventory=await invoke('get_sitemap_inventory');check(inventory.locations.length===3&&inventory.files.length===3,'Selected roots and a nested child produce one combined deduplicated inventory');
  check(inventory.locations.some(value=>decodeURI(value).endsWith('/שלום'))&&requests.filter(value=>value==='/shared.xml').length===1,'Nested shared sitemap is fetched once and Hebrew URLs survive retrieval');
  check(!requests.some(value=>value.includes('external')||value.includes('invalid-relative')),'Unselected external and invalid declarations were never requested');
  await table.getByRole('checkbox').nth(1).uncheck();await section.getByRole('button',{name:'Retrieve selected sitemaps (1)',exact:true}).click();await section.getByText(/2 entries ·/).waitFor();inventory=await invoke('get_sitemap_inventory');check(inventory.locations.length===2,'Actual checkbox selection controls the imported sitemap roots');
  const preserved=JSON.stringify(inventory);mode='gzip';await discover.click();await section.getByText(/3 discovered sitemap URLs/).waitFor();check(JSON.stringify(await invoke('get_sitemap_inventory'))===preserved,'Gzip robots discovery preserves the previous membership until explicit retrieval');
  mode='bad';await discover.click();await section.getByText(/2 discovered sitemap URLs/).waitFor();await section.getByRole('button',{name:'Retrieve selected sitemaps (2)',exact:true}).click();await section.getByRole('alert').filter({hasText:'HTTP 404'}).waitFor();check(JSON.stringify(await invoke('get_sitemap_inventory'))===preserved,'A later failing selected root preserves the complete previous inventory');
  mode='cycle';await discover.click();await section.getByText(/2 discovered sitemap URLs/).waitFor();await section.getByRole('button',{name:'Retrieve selected sitemaps (2)',exact:true}).click();await section.getByRole('alert').filter({hasText:'cycle'}).waitFor();check(JSON.stringify(await invoke('get_sitemap_inventory'))===preserved,'Cycle across independently selected roots is detected without replacing inventory');
  mode='empty';const emptyStart=requests.length;await discover.click();await section.getByText(/0 discovered sitemap URLs/).waitFor();check(requests.slice(emptyStart).join(',')==='/robots.txt'&&await section.getByRole('button',{name:'Retrieve selected sitemaps (0)',exact:true}).isDisabled(),'No declarations means zero discoveries without guessing a sitemap path');
  for(const kind of ['oversized','404','redirect','html']){mode=kind;await discover.click();await section.getByRole('alert').waitFor();check(JSON.stringify(await invoke('get_sitemap_inventory'))===preserved&&await section.getByRole('table',{name:'Discovered sitemaps'}).count()===0,'Rejected '+kind+' robots response preserves inventory and clears stale discovery');}
  check(!requests.includes('/unexpected-robots'),'Cross-origin robots redirect was not followed');
  await page.screenshot({path:path.join(output,'native-robots-discovery.png'),fullPage:true});fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,executable,db,requests},null,2));
 })}finally{await new Promise(resolve=>server.close(resolve))}
})().catch(error=>{console.error(error);process.exitCode=1});

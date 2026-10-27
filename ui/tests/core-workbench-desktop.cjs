// Native acceptance against an owned HTTP fixture; expected URL sets are independent of the UI model.
const fs=require('fs'),path=require('path'),http=require('http');
const {withNativeApp}=require('./native-driver.cjs');
const checks=[],traffic=[];
function check(ok,message){if(!ok)throw Error(message);checks.push(message);console.log('PASS: '+message)}
let origin;
const documents={
 '/':`<title>בית</title><meta name="description" content="תיאור בית"><h1>בית ראשי</h1><h2>כותרת משנה</h2>`,
 '/a':`<title>כפול</title><meta name="description" content="תיאור כפול"><h1>ראשון</h1><h1>שני</h1><h2>משנה א</h2><h2>משנה ב</h2><meta name="robots" content="noindex,follow">`,
 '/b':`<title>כפול</title><meta name="description" content="תיאור כפול"><h1>ראשון</h1><meta name="robots" content="index,nofollow"><link rel="alternate" hreflang="he" href="/a">`,
 '/missing':`<p>חסר מידע</p>`,
 '/folder/c':`<title>עמוד פנימי</title><meta name="description" content="תיאור פנימי"><h1>פנימי</h1><h2>משנה</h2>`,
};
const server=http.createServer(async(req,res)=>{
 traffic.push(req.url);res.setHeader('Connection','close');
 if(req.url==='/robots.txt'){res.setHeader('Content-Type','text/plain');return res.end('User-agent: *\nAllow: /')}
 if(req.url==='/redirect'){res.writeHead(302,{Location:'/folder/c'});return res.end()}
 if(req.url==='/absent'){res.writeHead(404,{'Content-Type':'text/plain'});return res.end('missing')}
 if(req.url==='/no-alt.png'||req.url==='/alt.png'){res.setHeader('Content-Type','image/png');return res.end(Buffer.from('89504e470d0a1a0a','hex'))}
 if(!Object.hasOwn(documents,req.url)){res.writeHead(404);return res.end()}
 res.setHeader('Content-Type','text/html; charset=utf-8');res.setHeader('X-Own-Fixture','workbench');
 if(req.url==='/folder/c')res.setHeader('Content-Security-Policy',"default-src 'self'");
 if(req.url==='/')await new Promise(resolve=>setTimeout(resolve,700));
 const links=req.url==='/'?Object.keys(documents).filter(x=>x!=='/').concat(['/redirect','/absent']).map(x=>`<a href="${x}">${x}</a>`).join('')+'<img src="/no-alt.png"><img src="/alt.png" alt="תמונה">':'';
 res.end('<!doctype html><html lang="he"><head>'+documents[req.url]+'</head><body>'+links+'</body></html>');
});
(async()=>{await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));origin='http://127.0.0.1:'+server.address().port;
try{await withNativeApp(async({page,invoke,output,executable})=>{
 const db=path.join(output,'workbench-'+Date.now()+'.sqlite');
 await page.getByLabel('Website URL',{exact:true}).fill(origin+'/');
 await page.getByRole('button',{name:'Crawl configuration',exact:true}).click();
 await page.getByLabel('Maximum URLs (0 = unlimited)',{exact:true}).fill('0');
 await page.getByLabel('Delay between page dispatches (ms)',{exact:true}).fill('0');
 await page.getByLabel('Project SQLite path (optional)',{exact:true}).fill(db);
 await page.getByRole('button',{name:'Done',exact:true}).click();
 await page.getByRole('button',{name:/Start/}).click();
 await page.locator('.crawl-state').getByText('Crawling',{exact:false}).waitFor();
 check((await invoke('get_snapshot')).running,'Actual Start exposes running progress before the fixture completes');
 let snapshot;for(let i=0;i<200;i++){snapshot=await invoke('get_snapshot');if(!snapshot.running)break;await page.waitForTimeout(100)}
 check(!snapshot.running&&!snapshot.error&&snapshot.pages.length===9,'Unlimited actual HTTP crawl records all nine fixture URLs without error');
 await page.waitForTimeout(500);
 const categories=page.getByRole('tablist',{name:'Result categories'}),rows=page.locator('.grid-wrap tbody tr.data-row');
 async function select(category,filter='All',search=''){
  await categories.getByRole('tab',{name:category,exact:true}).click();
  await page.locator('.filterbar select').selectOption(filter);
  await page.getByLabel('Search results',{exact:true}).fill(search);
  await page.waitForTimeout(60);
 }
 async function paths(){return rows.locator('td:nth-child(2)').allTextContents()}
 async function expectSet(category,filter,expected){await select(category,filter);const actual=(await paths()).map(x=>new URL(x).pathname).sort();check(JSON.stringify(actual)===JSON.stringify([...expected].sort()),category+' / '+filter+' displays the independent expected URL set')}
 await expectSet('Response Codes','Success (2xx)',['/','/a','/b','/missing','/folder/c','/no-alt.png','/alt.png']);
 await expectSet('Response Codes','Redirects (3xx)',['/redirect']);
 await expectSet('Response Codes','Errors (4xx–5xx)',['/absent']);
 await expectSet('Page Titles','Missing',['/missing']);
 await expectSet('Page Titles','Duplicate',['/a','/b']);
 await expectSet('Meta Description','Duplicate',['/a','/b']);
 await expectSet('Meta Description','Missing',['/missing']);
 await expectSet('H1','Missing',['/missing']);
 await expectSet('H2','Missing',['/b','/missing']);
 await expectSet('H1','Multiple',['/a']);
 await expectSet('H2','Multiple',['/a']);
 await expectSet('Images','Missing alt text',['/no-alt.png']);
 await expectSet('Directives','Noindex',['/a']);
 await expectSet('Directives','Nofollow',['/b']);
 await expectSet('Hreflang','Has hreflang',['/b']);
 await expectSet('Security','HTTP URLs',snapshot.pages.map(p=>new URL(p.url).pathname));
 await select('Internal','All','עמוד פנימי');
 check(await rows.count()===1&&(await paths())[0]===origin+'/folder/c','Actual search finds saved Hebrew metadata with one matching URL');
 await select('Internal');
 const addressHeader=page.locator('th[data-column="url"] button').first();
 await addressHeader.click();let sorted=await paths();
 check(await page.locator('th[data-column="url"]').getAttribute('aria-sort')==='descending'&&sorted.every((value,index)=>index===0||sorted[index-1].localeCompare(value,undefined,{numeric:true,sensitivity:'base'})>=0),'Address column sorts all fixture rows descending');
 await addressHeader.click();sorted=await paths();
 check(await page.locator('th[data-column="url"]').getAttribute('aria-sort')==='ascending'&&sorted.every((value,index)=>index===0||sorted[index-1].localeCompare(value,undefined,{numeric:true,sensitivity:'base'})<=0),'Address column sorts all fixture rows ascending');
 await rows.filter({hasText:origin+'/a'}).first().click();
 await page.getByRole('tab',{name:'URL Details',exact:true}).click();
 check((await page.locator('#url-details-panel').innerText()).includes('כפול'),'Selecting a row populates its Hebrew URL Details');
 await page.getByRole('tab',{name:'HTTP Headers',exact:true}).click();
 check((await page.locator('#url-details-panel').innerText()).includes('workbench'),'HTTP Headers pane shows the selected server-observed fixture header');
 await page.getByRole('tab',{name:'Headings',exact:true}).click();
 const headings=await page.locator('#url-details-panel').innerText();check(headings.includes('ראשון')&&headings.includes('שני')&&headings.includes('משנה ב'),'Headings pane retains all selected H1 and H2 occurrences');
 await rows.first().focus();await page.keyboard.press('End');
 check((await rows.last().getAttribute('class')).includes('selected'),'Keyboard End selects the final result');
 await page.keyboard.press('Home');
 check((await rows.first().getAttribute('class')).includes('selected'),'Keyboard Home selects the first result');
 await page.waitForFunction(()=>document.activeElement?.getAttribute('data-row-index')==='0');
 await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 await page.keyboard.press('Control+l');await page.waitForFunction(()=>document.activeElement?.id==='crawl-address');check(true,'Ctrl+L focuses the crawl address');
 await rows.first().focus();await page.evaluate(async()=>{document.querySelector('.grid-wrap tr.data-row').dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));document.dispatchEvent(new KeyboardEvent('keydown',{key:'l',ctrlKey:true,bubbles:true}));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))});check(await page.getByLabel('Website URL',{exact:true}).evaluate(element=>element===document.activeElement),'Rapid row navigation followed by Ctrl+L preserves address focus after deferred table updates');
 await rows.first().focus();await page.evaluate(async()=>{const row=document.querySelector('.grid-wrap tr.data-row');for(const key of ['End','Home'])row.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true}));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))});check(await rows.first().evaluate(element=>element===document.activeElement&&element.getAttribute('aria-current')==='true'),'The latest rapid row-navigation request wins and exposes the current row semantically');
 await page.evaluate(async()=>{document.querySelector('.grid-wrap tr.data-row').dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));document.querySelector('[aria-label="Search results"]').focus();await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))});check(await page.getByLabel('Search results',{exact:true}).evaluate(element=>element===document.activeElement),'Deferred row navigation does not steal focus from the search control');
 await page.getByRole('button',{name:'עברית',exact:true}).click();await rows.first().focus();await page.evaluate(async()=>{document.querySelector('.grid-wrap tr.data-row').dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));document.dispatchEvent(new KeyboardEvent('keydown',{key:'l',ctrlKey:true,bubbles:true}));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))});check(await page.locator('.app').getAttribute('dir')==='rtl'&&await page.getByLabel('Website URL',{exact:true}).evaluate(element=>element===document.activeElement),'Rapid navigation and address focus remain correct in the actual Hebrew RTL workbench');await page.getByRole('button',{name:'English',exact:true}).click();

 await rows.filter({hasText:origin+'/a'}).first().click({button:'right'});
 await page.getByRole('button',{name:'Inspect raw crawl data',exact:true}).click();
 check((await page.locator('#url-details-panel').innerText()).includes('כפול'),'Actual context-menu inspection opens the selected row raw data');
 await page.locator('#url-details-panel').getByRole('tab',{name:'Issues',exact:true}).click();
 const selected=snapshot.pages.find(p=>p.url===origin+'/a'),issueText=await page.locator('#url-details-panel').innerText();
 check(selected.issues.length>0&&selected.issues.every(issue=>issueText.includes(typeof issue==='string'?issue:JSON.stringify(issue))),'Issues pane displays every recorded issue for the selected URL');
 await select('Page Titles','Duplicate');
 const csv=path.join(output,'כותרות-כפולות.csv');
 await page.getByRole('button',{name:'↥ Export CSV',exact:true}).click();
 await page.getByRole('dialog').getByRole('textbox').fill(csv);
 await page.getByRole('button',{name:'Export',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('.notice')?.textContent.includes('Saved:'));
 const {spawnSync}=require('child_process'),parsed=spawnSync(process.env.FITIG_PYTHON||'python',['-X','utf8','-c',"import csv,sys\nr=list(csv.DictReader(open(sys.argv[1],encoding='utf-8-sig',newline='')))\nassert len(r)==2\nassert {x['Address'] for x in r}=={sys.argv[2]+'/a',sys.argv[2]+'/b'}\nassert all(x['Title']=='כפול' for x in r)",csv,origin],{encoding:'utf8'});
 check(parsed.status===0,'Independent Python CSV parser confirms the filtered two-row export and Hebrew titles');
 const overview=page.locator('#crawl-overview-panel');
 const metrics=await overview.locator('.metric').evaluateAll(nodes=>Object.fromEntries(nodes.map(n=>[n.querySelector('span').textContent,n.querySelector('b').textContent])));
 check(metrics['Success (2xx)']==='7'&&metrics['Redirection (3xx)']==='1'&&metrics['Client / server errors']==='1'&&metrics.Queued==='0'&&metrics['Unique addresses']==='9','Overview counters agree with all independently known response groups and queue completion');
 const before=fs.readFileSync(db),requests=traffic.length;
 await page.keyboard.press('F5');await page.waitForTimeout(400);
 check(fs.readFileSync(db).equals(before)&&traffic.length===requests,'F5 refresh reads saved observations without mutating the database or requesting pages');
 await page.screenshot({path:path.join(output,'native-workbench.png'),fullPage:true});
 fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,executable,db,pages:snapshot.pages.length,metrics,note:'Owned HTTP fixture acceptance, not complete competitor filter/menu parity.'},null,2));
})}finally{await new Promise(resolve=>server.close(resolve))}})().catch(error=>{console.error(error);process.exitCode=1});

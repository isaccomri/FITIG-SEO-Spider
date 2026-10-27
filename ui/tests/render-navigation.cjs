// Actual WebView2 acceptance: owns its fixture server, app, profile and databases.
const fs = require('fs'), path = require('path'), http = require('http'), zlib=require('zlib');
const {spawn} = require('child_process');
const {chromium} = require(process.env.FITIG_PLAYWRIGHT_MODULE || 'playwright');
const executable = path.resolve(process.env.FITIG_DESKTOP_EXECUTABLE);
const output = path.resolve(process.env.FITIG_ACCEPTANCE_DIR);
const port = Number(process.env.FITIG_CDP_PORT || 9267);
fs.mkdirSync(output, {recursive:true});
const checks = [], requests = [];
function check(condition, message) { if (!condition) throw Error(message); checks.push(message); console.log('PASS: ' + message); }
const server = http.createServer((req,res) => {
  requests.push(req.url); res.setHeader('Connection','close');
  res.setHeader('Content-Type','text/html; charset=utf-8');
  if (req.url === '/robots.txt') return res.end('User-agent: *\nAllow: /');
  const root='http://127.0.0.1:'+server.address().port,ns='http://www.sitemaps.org/schemas/sitemap/0.9';
  const index=child=>`<sitemapindex xmlns="${ns}"><sitemap><loc>${root}${child}</loc></sitemap></sitemapindex>`;
  if(req.url==='/sitemap-redirect'){res.writeHead(302,{Location:'/index.xml'});return res.end()}
  if(req.url==='/index.xml')return res.end(index('/nested.xml'));
  if(req.url==='/nested.xml')return res.end(index('/child.xml.gz'));
  if(req.url==='/child.xml.gz'){
    res.setHeader('Content-Encoding','gzip');
    const xml=`<urlset xmlns="${ns}"><url><loc>${root}/legacy</loc></url><url><loc>${root}/עברית</loc></url></urlset>`;
    return res.end(zlib.gzipSync(zlib.gzipSync(Buffer.from(xml))));
  }
  if(req.url==='/cycle.xml')return res.end(index('/cycle.xml'));
  if(req.url==='/cross.xml')return res.end(`<sitemapindex xmlns="${ns}"><sitemap><loc>http://localhost:${server.address().port}/do-not-fetch.xml</loc></sitemap></sitemapindex>`);
  if(req.url==='/dtd.xml')return res.end('<!DOCTYPE x [<!ENTITY x "x">]><urlset/>');
  if(req.url==='/bad-http-gzip.xml'){res.setHeader('Content-Encoding','gzip');return res.end('<urlset/>')}
  if (req.url === '/start') return res.end('<title>HTTP source</title><script>location.replace("/moved/page")</script>');
  if (req.url === '/moved/page') return res.end('<title>אחרי מעבר</title><body><a href="next">next</a><img src="photo.png" alt="צילום"></body>');
  if (req.url === '/base') return res.end('<base href="/קטלוג/"><title>בסיס</title><link rel="canonical" href="item"><body><a href="item">item</a></body>');
  if (req.url === '/external-start') return res.end('<script>location.replace("http://localhost:'+server.address().port+'/moved/page")</script>');
  if (req.url === '/legacy') {
    res.setHeader('Content-Type','text/html; charset=windows-1255');
    const html='<title>שלום</title><body><h1>עברית</h1><a href="/עברית">קישור</a></body>';
    return res.end(Buffer.from([...html].map(char=>{const code=char.charCodeAt(0);return code>=0x5d0&&code<=0x5ea?code-0x5d0+0xe0:code})));
  }
  res.statusCode = 404; res.end('missing');
});
async function run() {
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  const origin = 'http://127.0.0.1:' + server.address().port;
  const app = spawn(executable, [], {cwd:path.dirname(executable), stdio:'inherit', windowsHide:false,
    env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port='+port,
      WEBVIEW2_USER_DATA_FOLDER:path.join(output,'owned-profile-'+Date.now())}});
  let browser;
  try {
    let endpoint;
    for (let attempt=0;attempt<60;attempt++) {
      if (app.exitCode !== null) throw Error('App exited: '+app.exitCode);
      try { endpoint=(await (await fetch('http://127.0.0.1:'+port+'/json/version', {signal:AbortSignal.timeout(1000)})).json()).webSocketDebuggerUrl; } catch {}
      if (endpoint) break;
      await new Promise(resolve=>setTimeout(resolve,500));
    }
    if (!endpoint) throw Error('WebView2 CDP endpoint unavailable');
    browser = await chromium.connectOverCDP(endpoint);
    const page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(30000);
    await page.getByText('Engine connected',{exact:true}).waitFor();
    const invoke=(name,args)=>page.evaluate(([name,args])=>window.__TAURI_INTERNALS__.invoke(name,args),[name,args]);
    const observations=[];
    for (const route of ['/start','/base','/external-start','/legacy']) {
      const db=path.join(output,route.slice(1)+'-'+Date.now()+'.sqlite');
      await invoke('start_crawl',{url:origin+route,limit:1,delayMs:0,path:db,resume:false,mode:'spider',urls:null,
        profile:{store_html:true,rendering:{enabled:route!=='/legacy',binary:'',wait_ms:700,timeout_ms:20000}}});
      let snapshot;
      for (let attempt=0;attempt<100;attempt++) {
        snapshot=await invoke('get_snapshot');if(!snapshot.running)break;
        await new Promise(resolve=>setTimeout(resolve,300));
      }
      check(!snapshot.running&&!snapshot.error,route+' native crawl completed');
      const record=snapshot.pages[0];
      if(route!=='/legacy')check(record?.rendered && !record.render_error,route+' Chromium rendered successfully');
      check(record.url===origin+route && record.status===200,route+' original HTTP identity preserved');
      if(route==='/start') {
        check(record.rendered_url===origin+'/moved/page','JavaScript navigation final URL retained');
        check(record.links[0].url===origin+'/moved/next','Relative link uses browser location');
        check(record.images[0][0]===origin+'/moved/photo.png','Relative image uses browser location');
        const raw=await invoke('get_source',{url:record.url,rendered:false});
        const dom=await invoke('get_source',{url:record.url,rendered:true});
        check(raw.includes('HTTP source')&&dom.includes('אחרי מעבר'),'Raw and final Hebrew DOM stored separately');
        await page.locator('.grid-wrap tbody tr').first().click();
        await page.getByRole('tab',{name:'URL Details',exact:true}).click();
        check(await page.locator('.detail-content').innerText().then(text=>text.includes('/moved/page')),'Final renderer URL visible in URL Details');
        await page.screenshot({path:path.join(output,'rendered-url-details.png'),fullPage:true});
      } else if(route==='/base') {
        check(decodeURI(record.canonical)===origin+'/קטלוג/item','Hebrew base resolves canonical');
        check(record.links[0].url===record.canonical,'Hebrew base resolves link consistently');
      } else if(route==='/external-start') {
        check(record.rendered_url.startsWith('http://localhost:'),'Cross-origin browser navigation retained');
        check(record.links[0].internal===false&&snapshot.queued===0,'Cross-origin final page does not widen crawl scope');
      } else {
        check(!record.rendered&&record.title==='שלום'&&record.headings[0][0]==='עברית','Native HTTP extraction decodes Windows-1255 Hebrew');
        check(record.charset==='windows-1255'&&record.charset_source==='HTTP header'&&!record.decoding_errors,'Detected charset and decoder evidence persisted');
        check(decodeURI(record.links[0].url)===origin+'/עברית'&&record.links[0].text==='קישור','Legacy Hebrew URL and anchor text preserved');
        const csv=path.join(output,'legacy-hebrew.csv');await invoke('export_report',{path:csv,format:'csv'});
        check(fs.readFileSync(csv,'utf8').includes('שלום'),'Decoded Hebrew survives native CSV export');
      }
      observations.push({db,snapshot});
    }
    await page.getByRole('tablist',{name:'Result categories'}).getByRole('tab',{name:'Sitemaps',exact:true}).click();
    await page.getByLabel('Sitemap URL',{exact:true}).fill(origin+'/sitemap-redirect');
    await page.getByRole('button',{name:'Retrieve sitemap',exact:true}).click();
    await page.getByText(/2 entries ·/).waitFor();
    const inventory=await invoke('get_sitemap_inventory');
    check(inventory.locations.length===2&&inventory.files.length===3,'Native remote sitemap follows same-origin redirect and nested indexes');
    check(inventory.locations.some(url=>decodeURI(url).endsWith('/עברית')),'HTTP gzip plus gzip sitemap preserves Hebrew location');
    for(const [route,message]of [['/missing-map.xml','HTTP 404'],['/cycle.xml','cycle'],['/cross.xml','Cross-origin'],['/dtd.xml','DTD'],['/bad-http-gzip.xml','Invalid HTTP gzip']]){
      await page.getByLabel('Sitemap URL',{exact:true}).fill(origin+route);
      await page.getByRole('button',{name:'Retrieve sitemap',exact:true}).click();
      await page.getByRole('alert').filter({hasText:message}).waitFor();
      const after=await invoke('get_sitemap_inventory');
      check(JSON.stringify(after)===JSON.stringify(inventory),'Rejected '+route+' without replacing saved inventory');
    }
    check(!requests.includes('/do-not-fetch.xml'),'Cross-origin index child was never requested');
    await page.screenshot({path:path.join(output,'sitemap-retrieval.png'),fullPage:true});
    const orphanUrl=new URL(origin+'/יתום').href;
    await invoke('save_sitemap_inventory',{inventory:{...inventory,locations:[...inventory.locations,orphanUrl]}});
    const linkReport=await invoke('analyze_link_graph');
    const candidate=linkReport.rows.find(row=>row.url===orphanUrl);
    check(linkReport.scored_pages===1&&linkReport.converged,'Native link analysis scores the successful internal HTML graph');
    check(candidate?.orphan_candidate&&candidate.status===null&&candidate.score===null,'Inventory-only orphan candidate has no fabricated response or score');
    check(linkReport.rows.find(row=>decodeURI(row.url).endsWith('/עברית'))?.reachable===true,'Observed hyperlink path is retained for an uncrawled inventory URL');
    await page.getByRole('button',{name:'Crawl Analysis',exact:true}).click();
    await page.getByRole('button',{name:'Link scoring and orphan candidates…',exact:true}).click();
    await page.getByLabel('Only orphan candidates',{exact:true}).check();
    await page.getByLabel('Search link analysis',{exact:true}).fill('יתום');
    check(await page.locator('[aria-label="Link scoring and orphan candidates"] tbody tr').count()===1,'Native link report filters orphan candidates using readable Hebrew search');
    const linkCsv=path.join(output,'orphan-link-analysis.csv');
    await page.getByLabel('Link analysis CSV path',{exact:true}).fill(linkCsv);
    await page.getByRole('button',{name:'Export link analysis',exact:true}).click();
    await page.getByText('Saved: '+linkCsv,{exact:true}).waitFor();
    const exported=fs.readFileSync(linkCsv,'utf8');
    check(exported.includes('יתום')&&exported.includes('Unknown')&&exported.includes(orphanUrl),'Native filtered link report CSV retains Hebrew, unknown status and original URL');
    await page.screenshot({path:path.join(output,'orphan-link-analysis.png'),fullPage:true});
    await invoke('save_sitemap_inventory',{inventory:{...inventory,locations:[...inventory.locations,orphanUrl,...Array.from({length:205},(_,index)=>origin+'/orphan-'+index)]}});
    await page.getByLabel('Search link analysis',{exact:true}).fill('');
    await page.getByRole('button',{name:'Recalculate link analysis',exact:true}).click();
    await page.getByText('206 results · page 1 of 2',{exact:true}).waitFor();
    check(await page.locator('[aria-label="Link scoring and orphan candidates"] tbody tr').count()===200,'Link report limits rendered rows to a 200-row page');
    await page.getByRole('button',{name:'Next results',exact:true}).click();
    check(await page.locator('[aria-label="Link scoring and orphan candidates"] tbody tr').count()===6,'Next report page exposes the remaining candidates');
    const pagedCsv=path.join(output,'all-orphan-candidates.csv');
    await page.getByLabel('Link analysis CSV path',{exact:true}).fill(pagedCsv);
    await page.getByRole('button',{name:'Export link analysis',exact:true}).click();
    await page.getByText('Saved: '+pagedCsv,{exact:true}).waitFor();
    check(fs.readFileSync(pagedCsv,'utf8').trim().split(/\r?\n/).length===207,'Paginated report CSV exports all 206 filtered rows');
    fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({success:true,checks,requests,observations,inventory,linkReport},null,2));
  } finally {
    if(browser) await browser.close();
    if(app.exitCode===null)app.kill();
    await new Promise(resolve=>server.close(resolve));
  }
}
run().catch(error=>{console.error(error.stack);server.close();process.exitCode=1});

import {directiveRestriction} from './directives-model';
import {status,type PageRecord} from './model';
import {displayUrl} from './model';
export type SitemapFinding={url:string;status:number|string;membership:string;issues:string};
export const SITEMAP_PAGE_SIZE=200;
export function selectSitemapFindings(rows:SitemapFinding[],filter:string,search:string,issuesOnly=false){const query=search.trim().toLocaleLowerCase();return rows.filter(row=>{
 if(issuesOnly&&!row.issues)return false;
 if(filter==='In sitemap'&&row.membership!=='In sitemap')return false;
 if(filter==='Missing from sitemap'&&row.membership!=='Missing from sitemap')return false;
 if(filter==='Not crawled'&&row.status!=='Not crawled')return false;
 if(filter==='Duplicate locations'&&!row.issues.split(' · ').includes('Duplicate location'))return false;
 return !query||[row.url,displayUrl(row.url),String(row.status),row.membership,row.issues].some(value=>value.toLocaleLowerCase().includes(query));
})}
export function normalizeLocation(raw:string){const u=new URL(raw);if(!['http:','https:'].includes(u.protocol)||u.hash||u.username||u.password)throw Error('Locations must be absolute HTTP(S) URLs without credentials or fragments.');return u.href}
export function auditSitemap(locations:string[],pages:PageRecord[]):SitemapFinding[]{
 const counts=new Map<string,number>();for(const raw of locations){const url=normalizeLocation(raw);counts.set(url,(counts.get(url)??0)+1)}
 const observed=new Map(pages.map(p=>[normalizeLocation(p.url),p]));
 const findings:SitemapFinding[]=[];
 for(const [url,count]of counts){const p=observed.get(url),issues:string[]=[];if(count>1)issues.push('Duplicate location');if(!p)issues.push('Not crawled');else{const code=status(p);if(!code)issues.push('No HTTP response');else if(code<200||code>=300)issues.push(code<400?'Redirect':'HTTP error');if(!String(p.content_type??'').includes('text/html'))issues.push('Non-HTML');if(p.html_extracted===false)issues.push('HTML extraction unavailable');if(directiveRestriction(p,'googlebot','noindex'))issues.push('Noindex');if(p.canonical&&normalizeLocation(p.canonical)!==url)issues.push('Non-self canonical')}
 findings.push({url,status:p?status(p)||'No response':'Not crawled',membership:'In sitemap',issues:issues.join(' · ')})}
 for(const [url,p]of observed)if(p.internal!==false&&p.html_extracted!==false&&!counts.has(url)&&status(p)>=200&&status(p)<300&&String(p.content_type??'').includes('text/html')&&!directiveRestriction(p,'googlebot','noindex')&&(!p.canonical||normalizeLocation(p.canonical)===url))findings.push({url,status:status(p),membership:'Missing from sitemap',issues:'Indexable HTML missing from supplied sitemap'});
 return findings;
}
export type SitemapDocument={kind:'urlset'|'sitemapindex';locations:string[]};
export function parseSitemapDocument(xml:string):SitemapDocument{
 if(/<!DOCTYPE|<!ENTITY/i.test(xml))throw Error('DTD and entity declarations are not supported.');
 const doc=new DOMParser().parseFromString(xml,'application/xml');if(doc.getElementsByTagName('parsererror').length)throw Error('Invalid XML');
 const root=doc.documentElement,namespace='http://www.sitemaps.org/schemas/sitemap/0.9';if(!['urlset','sitemapindex'].includes(root.localName)||root.namespaceURI!==namespace)throw Error('Expected a sitemap urlset or sitemapindex with the standard sitemap namespace.');
 const kind=root.localName as SitemapDocument['kind'];const entries=Array.from(root.children).filter(e=>e.localName===(kind==='urlset'?'url':'sitemap')&&e.namespaceURI===namespace);if(entries.length>50000)throw Error('A single sitemap cannot exceed 50,000 entries.');
 return {kind,locations:entries.map(entry=>{const loc=Array.from(entry.children).filter(e=>e.localName==='loc'&&e.namespaceURI===namespace);if(loc.length!==1||loc[0].children.length||!loc[0].textContent?.trim())throw Error('Every entry must contain exactly one non-empty text loc.');return normalizeLocation(loc[0].textContent.trim())})};
}
export function parseSitemapXml(xml:string):string[]{const document=parseSitemapDocument(xml);if(document.kind==='sitemapindex')throw Error('This is a sitemap index. Import it together with its child sitemap files.');return document.locations}
export type SitemapFile={name:string;document:SitemapDocument};
export function reconcileSitemapFiles(files:SitemapFile[]){
 const names=new Map<string,SitemapFile>();for(const file of files){if(names.has(file.name))throw Error('Ambiguous duplicate sitemap filename: '+file.name);names.set(file.name,file)}
 const referenced=new Set<string>(),missing=new Set<string>();
 const filename=(url:string)=>decodeURIComponent(new URL(url).pathname.split('/').pop()??'');
 for(const file of files)if(file.document.kind==='sitemapindex')for(const url of file.document.locations){const name=filename(url);if(names.has(name))referenced.add(name);else missing.add(url)}
 let roots=files.filter(f=>!referenced.has(f.name));if(!roots.length&&files.length)roots=[files[0]];
 const locations:string[]=[],visited=new Set<string>();
 function visit(file:SitemapFile,parents:Set<string>){if(parents.has(file.name))throw Error('Sitemap index cycle: '+file.name);if(visited.has(file.name))return;visited.add(file.name);if(file.document.kind==='urlset'){locations.push(...file.document.locations);if(locations.length>250000)throw Error('Import exceeds 250,000 total URL entries.');return}const next=new Set(parents).add(file.name);for(const url of file.document.locations){const child=names.get(filename(url));if(child)visit(child,next)}}
 for(const file of roots)visit(file,new Set());for(const file of files)if(!visited.has(file.name))visit(file,new Set());
 return {locations,missing:[...missing],files:files.map(f=>f.name)};
}

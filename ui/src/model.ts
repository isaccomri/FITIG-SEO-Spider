import {directiveCell,directiveRestriction} from './directives-model.ts';
export type CrawlLink={url:string;text?:string;rel?:string;kind?:string;internal?:boolean;alt?:string;target?:string;position?:string;path_type?:string};
export type PageRecord = {url:string; status?:number; status_code?:number; title?:string; description?:string; meta_description?:string; content_type?:string; depth?:number; canonical?:string; headings?:Record<string,string[]>|string[][]; h1?:string[]; links?:CrawlLink[]; inlinks?:unknown[]; outlinks?:unknown[]; issues?:unknown[]; word_count?:number; response_time_ms?:number; elapsed_ms?:number; size?:number; redirect?:string; images?:[string,string][]; json_ld?:string[]; directives?:string; hreflang?:[string,string][]; [key:string]:unknown};
export type Snapshot = {pages:PageRecord[]; running:boolean; error?:string; queued?:number; completed?:number; project_path?:string; [key:string]:unknown};
export const categories=['Internal','External','Security','Response Codes','URL','Page Titles','Meta Description','H1','H2','H3–H6','Content','Images','Canonicals','Directives','Hreflang','JavaScript','Links','Structured Data','Sitemaps','Custom Search','Custom Extraction','Mobile','Accessibility','Issues'];
export const unavailableCategories:Record<string,string>={};
export const DEFAULT_CRAWL_LIMIT=0;
export function parseCrawlLimit(value:string){const n=Number(value);return Number.isSafeInteger(n)&&n>=0?n:0}
export function displayUrl(value:string){return value.replace(/(?:%[89a-f][0-9a-f])+/gi,encoded=>{try{const decoded=decodeURIComponent(encoded);return /[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/.test(decoded)?encoded:decoded}catch{return encoded}})}
export function status(p:PageRecord){return Number(p.status_code ?? p.status ?? 0)}
export function issueText(p:PageRecord){return (p.issues??[]).map(x=>typeof x==='string'?x:JSON.stringify(x)).join(' · ')}
export function headings(p:PageRecord,n:number){return Array.isArray(p.headings)?p.headings[n-1]??[]:p.headings?.[`h${n}`]??(n===1?p.h1??[]:[])}
export function getCell(p:PageRecord,key:string):string|number {
 const aliases:Record<string,unknown>={robots_noindex:directiveCell(p,'*','noindex'),googlebot_noindex:directiveCell(p,'googlebot','noindex'),googlebot_nofollow:directiveCell(p,'googlebot','nofollow'),bingbot_noindex:directiveCell(p,'bingbot','noindex'),status:status(p)||'',description:p.description??p.meta_description,issues:issueText(p),url_length:p.url.length,title_length:(p.title??'').length,description_length:(p.description??p.meta_description??'').length,response_time_ms:p.response_time_ms??p.elapsed_ms,size_bytes:p.size_bytes??p.size,redirect_url:p.redirect_url??p.redirect,image_count:p.images?.length??0,link_count:p.links?.length??0,json_ld_count:p.json_ld?.length??0,hreflang_count:p.hreflang?.length??0};
 for(let i=1;i<=6;i++)aliases[`h${i}`]=headings(p,i);
 const value=Object.hasOwn(aliases,key)?aliases[key]:p[key];const cell=Array.isArray(value)?value.map(v=>Array.isArray(v)?v.join(' → '):typeof v==='object'?JSON.stringify(v):v).join(' | '):value==null?'':typeof value==='object'?JSON.stringify(value):String(value);return ['url','rendered_url','canonical','redirect_url','images','links','hreflang'].includes(key)?displayUrl(cell):cell
}
export function filtersFor(category:string){if(category==='Mobile')return ['All','Audited','Findings','Audit unavailable','Not recorded','Truncated details'];if(category==='Accessibility')return ['All','Audited','Violations','Needs review','Audit unavailable','Not recorded','Truncated details'];if(['Page Titles','Meta Description','H1','H2','Canonicals'].includes(category))return ['H1','H2'].includes(category)?['All','Missing','Duplicate','Multiple']:['All','Missing','Duplicate'];if(category==='Response Codes')return ['All','Success (2xx)','Redirects (3xx)','Errors (4xx–5xx)'];if(category==='Security')return ['All','HTTP URLs','Missing HSTS','Missing CSP'];if(category==='Images')return ['All','Missing alt text'];if(category==='Directives')return ['All','Noindex','Nofollow'];if(category==='Content')return ['All','Duplicate','Duplicate response body','Low word count'];if(category==='JavaScript')return ['All','Has issues','Rendered pages','Console errors','Rendering failed','Custom script results','Custom script errors'];if(category==='Hreflang')return ['All','Has hreflang','Missing'];return ['All','Has issues']}
function host(url:string){try{return new URL(url).host}catch{return ''}}

export function inCrawlScope(pages:PageRecord[],raw:string):boolean {
 const base=pages.find(page=>page.internal!==false)??pages[0];if(!base)return false;
 try {const seed=new URL(base.url),target=new URL(raw);if(target.origin===seed.origin)return true;
 const setting=String(base.subdomain_root??'').trim();if(!setting)return false;
 const root=new URL('https://'+setting+'/').hostname.replace(/\.$/,'').toLowerCase(),host=target.hostname.replace(/\.$/,'').toLowerCase();
 return target.protocol===seed.protocol&&target.port===seed.port&&(host===root||host.endsWith('.'+root));
 }catch{return false}
}
export function resultInventory(pages:PageRecord[],category:string):PageRecord[]{
 if(category==='Links'){const incoming=new Map<string,Set<string>>();for(const page of pages)for(const link of page.links??[])if(link.kind==='anchor'){const sources=incoming.get(link.url)??new Set<string>();sources.add(page.url);incoming.set(link.url,sources)}return pages.map(page=>({...page,unique_inlinks:incoming.get(page.url)?.size??0,unique_internal_outlinks:new Set((page.links??[]).filter(link=>link.kind==='anchor'&&link.internal!==false).map(link=>link.url)).size,unique_external_outlinks:new Set((page.links??[]).filter(link=>link.kind==='anchor'&&link.internal===false).map(link=>link.url)).size}))}
 const actual=new Map(pages.map(p=>[p.url,p])),references=new Map<string,{sources:Set<string>;alts:string[];kind:string}>();
 const add=(url:string,source:string,kind:string,alt='')=>{try{if(!['http:','https:'].includes(new URL(url).protocol))return}catch{return}const entry=references.get(url)??{sources:new Set<string>(),alts:[],kind};entry.sources.add(source);if(kind==='image')entry.alts.push(alt);references.set(url,entry)};
 for(const p of pages){for(const l of p.links??[])if(category==='External'||category==='JavaScript'&&l.kind==='script')add(l.url,p.url,l.kind??'anchor');for(const [url,alt]of p.images??[])if(category==='Images'||category==='External')add(url,p.url,'image',alt);if(category==='External'){if(p.canonical)add(p.canonical,p.url,'canonical');for(const [,url]of p.hreflang??[])add(url,p.url,'hreflang');const redirect=String(p.redirect??p.redirect_url??'');if(redirect)add(redirect,p.url,'redirect')}}
 if(category==='External'){const urls=new Set([...pages.filter(p=>p.internal===false||!inCrawlScope(pages,p.url)).map(p=>p.url),...references.keys()]);return [...urls].filter(url=>actual.get(url)?.internal===false||!inCrawlScope(pages,url)).map(url=>({...actual.get(url),url,internal:false,source_count:references.get(url)?.sources.size??0,discovered_type:references.get(url)?.kind??'',observation:actual.has(url)?'Fetched':'Discovered; not fetched'}))}
 if(category==='Images'||category==='JavaScript'){const mime=category==='Images'?'image/':'javascript',urls=new Set([...pages.filter(p=>String(p.content_type??'').includes(mime)).map(p=>p.url),...references.keys()]);return [...urls].map(url=>({...actual.get(url),url,image_resource:category==='Images',script_resource:category==='JavaScript',image_alt_texts:references.get(url)?.alts??[],image_inlinks:references.get(url)?.sources.size??0,source_count:references.get(url)?.sources.size??0,observation:actual.has(url)?'Fetched':'Discovered; not fetched'}))}
 return pages;
}
export function selectPages(pages:PageRecord[],category:string,filter:string,search:string){
 // Sitemaps uses its dedicated saved-inventory/reconciliation workspace rather than the generic page grid.
 if(category==='Sitemaps'||unavailableCategories[category])return [];
 const javascriptPages=category==='JavaScript'&&['Rendered pages','Console errors','Rendering failed','Custom script results','Custom script errors'].includes(filter);
 if(['External','Images','JavaScript','Links'].includes(category)&&!javascriptPages)pages=resultInventory(pages,category);
 const htmlCategory=['Page Titles','Meta Description','H1','H2','H3–H6','Content','Canonicals','Hreflang','Structured Data'].includes(category);
 // Preserve legacy records with unknown observations, but never diagnose a known non-HTML response as missing HTML metadata.
 if(htmlCategory)pages=pages.filter(page=>{if(page.html_extracted===false)return false;const mime=String(page.content_type??'').toLowerCase();return mime.includes('html')||!mime&&page.status==null&&page.status_code==null});
 const missingKey=category==='Page Titles'?'title':category==='Meta Description'?'description':category==='Canonicals'?'canonical':category==='H2'?'h2':category==='Hreflang'?'hreflang':category==='Content'?(filter==='Duplicate response body'?'response_hash':'content_hash'):'h1';
 const duplicateCounts=new Map<string,number>();if(filter==='Duplicate'||filter==='Duplicate response body'){for(const page of pages){const value=getCell(page,missingKey);if(value){const key=String(value);duplicateCounts.set(key,(duplicateCounts.get(key)??0)+1)}}}
 return pages.filter(p=>{const content=String(p.content_type??'');const html=content.includes('html');
 if(category==='Mobile'){const recorded=typeof p.mobile_findings==='number';if(filter==='Audited'&&!recorded)return false;if(filter==='Findings'&&Number(p.mobile_findings??0)<=0)return false;if(filter==='Audit unavailable'&&!p.mobile_error)return false;if(filter==='Not recorded'&&(recorded||Boolean(p.mobile_error)))return false;if(filter==='Truncated details'&&!p.mobile_truncated)return false;}
 if(category==='Accessibility'){const recorded=typeof p.accessibility_violations==='number';if(filter==='Audited'&&!recorded)return false;if(filter==='Violations'&&Number(p.accessibility_violations??0)<=0)return false;if(filter==='Needs review'&&Number(p.accessibility_review??0)<=0)return false;if(filter==='Audit unavailable'&&!p.accessibility_error)return false;if(filter==='Not recorded'&&(recorded||Boolean(p.accessibility_error)))return false;if(filter==='Truncated details'&&!p.accessibility_truncated)return false;}
 if(category==='Images'&&!p.image_resource)return false;
 if(category==='JavaScript'&&!p.script_resource&&!javascriptPages)return false;
 if(filter==='Rendered pages'&&!p.rendered)return false;
 if(filter==='Console errors'&&Number(p.console_errors??0)<=0)return false;
 if(filter==='Rendering failed'&&!p.render_error)return false;
 if(filter==='Custom script results'&&typeof p.javascript_count!=='number')return false;
 if(filter==='Custom script errors'&&Number(p.javascript_errors??0)<=0)return false;
 if(category==='Issues'&&!p.issues?.length)return false;
 if(category==='External'&&p.internal!==false&&p.is_internal!==false&&(!pages[0]||host(p.url)===host(pages[0].url)))return false;
 if(category==='Internal'&&(p.internal===false||p.is_internal===false))return false;
 if(['Page Titles','Meta Description','H1','H2','H3–H6','Content','Canonicals','Hreflang','Structured Data'].includes(category)&&content!==''&&!html)return false;
 if(category==='Structured Data'&&!p.json_ld?.length)return false;
 if(category==='Links'&&!p.links?.length)return false;
 if(filter==='Success (2xx)'&&(status(p)<200||status(p)>=300))return false;
 if(filter==='Errors (4xx–5xx)'&&status(p)<400)return false;
 if(filter==='Redirects (3xx)'&&(status(p)<300||status(p)>=400))return false;
 if(filter==='Missing'&&String(getCell(p,missingKey)).trim())return false;
 if(filter==='Duplicate'||filter==='Duplicate response body'){const value=getCell(p,missingKey);if(!value||(duplicateCounts.get(String(value))??0)<2)return false}
 if(filter==='Multiple'&&(!['H1','H2'].includes(category)||headings(p,category==='H2'?2:1).length<2))return false;
 if(filter==='Has issues'&&!p.issues?.length)return false;
 if(filter==='HTTP URLs'&&!p.url.startsWith('http:'))return false;
 const headers=p.headers as Record<string,string>|undefined;
 if(filter==='Missing HSTS'&&headers?.['strict-transport-security'])return false;
 if(filter==='Missing CSP'&&headers?.['content-security-policy'])return false;
 if(filter==='Missing alt text'&&!(p.image_alt_texts as string[]|undefined)?.some(alt=>!alt.trim()))return false;
 if(filter==='Noindex'&&!directiveRestriction(p,'googlebot','noindex'))return false;
 if(filter==='Nofollow'&&!directiveRestriction(p,'googlebot','nofollow'))return false;
 if(filter==='Low word count'&&(p.word_count??0)>=200)return false;
 if(filter==='Has hreflang'&&!p.hreflang?.length)return false;
 return !search||JSON.stringify(p).toLowerCase().includes(search.toLowerCase())||displayUrl(JSON.stringify(p)).toLowerCase().includes(search.toLowerCase())})
}
export function inlinksFor(pages:PageRecord[],url:string){return pages.flatMap(page=>(page.links??[]).filter(link=>link.url===url).map(link=>({source:page.url,...link})))}
export function columnsFor(category:string){const specific:Record<string,string[]>={Mobile:['mobile_findings','mobile_error','mobile_truncated'],Accessibility:['accessibility_violations','accessibility_nodes','accessibility_review','accessibility_error','accessibility_truncated'],'Page Titles':['title','title_length'],'Meta Description':['description','description_length'],H1:['h1'],H2:['h2'],'H3–H6':['h3','h4','h5','h6'],Content:['word_count','content_hash','response_hash'],Canonicals:['canonical'],Images:['content_type','image_alt_texts','image_inlinks','size_bytes','observation'],Security:['headers','issues'],'Response Codes':['redirect_url','response_time_ms'],URL:['url_length','depth'],Directives:['directives','robots_noindex','googlebot_noindex','googlebot_nofollow','bingbot_noindex'],Hreflang:['hreflang','hreflang_count'],JavaScript:['content_type','source_count','rendered','console_count','console_errors','javascript_count','javascript_errors','render_error','observation'],External:['content_type','source_count','discovered_type','observation'],Links:['link_count','unique_inlinks','unique_internal_outlinks','unique_external_outlinks'],'Structured Data':['json_ld_count','issues'],Issues:['issues']};return [...new Set(['url','status',...(specific[category]??['title','content_type']),'depth'])]}

import {inCrawlScope,displayUrl,type PageRecord,type CrawlLink} from './model';
export type LinkRow={id:string;link_type:string;from:string;to:string;anchor_text:string;alt_text:string;follow:boolean|null;rel:string;target:string;status:number|null;position:string;path_type:string;origin:string;internal:boolean};
export type LinkFilters={type?:string;origin?:string;follow?:string;scope?:string;unique?:boolean;search?:string};
const names:Record<string,string>={anchor:'Hyperlink',image:'Image',script:'JavaScript',stylesheet:'CSS',canonical:'Canonical',hreflang:'Hreflang',redirect:'HTTP Redirect',sitemap:'Sitemap'};
export function relationshipRows(pages:PageRecord[],url:string,direction:'Inlinks'|'Outlinks'):LinkRow[]{
 const byUrl=new Map(pages.map(p=>[p.url,p]));const rows:LinkRow[]=[];
 for(const page of pages){
  if(direction==='Outlinks'&&url&&page.url!==url)continue;
  const add=(link:CrawlLink,index:string)=>{if(direction==='Inlinks'&&link.url!==url)return;const kind=link.kind??'anchor';const nofollow=(page.directives??'').toLowerCase().split(/[\s,]+/).some(v=>v==='nofollow'||v==='none')||(link.rel??'').toLowerCase().split(/\s+/).includes('nofollow');const status=Number(byUrl.get(link.url)?.status_code??byUrl.get(link.url)?.status??0);rows.push({id:JSON.stringify([page.url,index,link.url]),link_type:names[kind]??kind,from:page.url,to:link.url,anchor_text:link.text??'',alt_text:link.alt??'',follow:kind==='anchor'?!nofollow:null,rel:link.rel??'',target:link.target??'',status:status>=100?status:null,position:link.position??'',path_type:link.path_type??'',origin:kind==='redirect'?'HTTP':kind==='sitemap'?'XML':'HTML',internal:link.internal??(typeof byUrl.get(link.url)?.internal==='boolean'?Boolean(byUrl.get(link.url)?.internal):inCrawlScope(pages,link.url))})};
  (page.links??[]).forEach((link,index)=>add(link,`link-${index}`));
  (page.images??[]).forEach(([target,alt],index)=>add({url:target,alt,kind:'image'},`image-${index}`));
  if(page.canonical)add({url:page.canonical,kind:'canonical'},'canonical');
  (page.hreflang??[]).forEach(([language,target],index)=>add({url:target,kind:'hreflang',text:language},`hreflang-${index}`));
  const redirect=String(page.redirect??page.redirect_url??'');if(redirect)add({url:redirect,kind:'redirect'},'redirect');
 }
 return rows;
}
export function filterLinkRows(rows:LinkRow[],filters:LinkFilters){const seen=new Set<string>();return rows.filter(row=>{
 if(filters.type&&row.link_type!==filters.type)return false;if(filters.origin&&row.origin!==filters.origin)return false;
 if(filters.follow==='follow'&&row.follow!==true)return false;if(filters.follow==='nofollow'&&row.follow!==false)return false;
 if(filters.scope==='internal'&&!row.internal)return false;if(filters.scope==='external'&&row.internal)return false;
 if(filters.search&&!displayUrl(JSON.stringify(row)).toLowerCase().includes(filters.search.toLowerCase())&&!JSON.stringify(row).toLowerCase().includes(filters.search.toLowerCase()))return false;
 if(filters.unique){const key=JSON.stringify([row.link_type,row.from,row.to]);if(seen.has(key))return false;seen.add(key)}return true;
})}
export function sortLinkRows(rows:LinkRow[],key:keyof LinkRow,asc:boolean){return [...rows].sort((a,b)=>String(a[key]??'').localeCompare(String(b[key]??''),undefined,{numeric:true})*(asc?1:-1))}
export const linkColumns:Array<[keyof LinkRow,string]>=[['link_type','Type'],['from','From'],['to','To'],['anchor_text','Anchor Text'],['alt_text','Alt Text'],['follow','Follow'],['rel','Rel'],['target','Target'],['status','Status Code'],['position','Link Position'],['path_type','Path Type'],['origin','Link Origin'],['internal','Internal']];
export function linkCell(row:LinkRow,key:keyof LinkRow){const value=row[key];if(value==null||value==='')return '—';if(key==='from'||key==='to')return displayUrl(String(value));if(typeof value==='boolean')return value?'Yes':'No';return String(value)}

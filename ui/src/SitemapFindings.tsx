import {NativeFileBrowse} from './NativeFileBrowse';
import React,{useMemo,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {displayUrl} from './model';
import {SITEMAP_PAGE_SIZE,selectSitemapFindings,type SitemapFinding} from './sitemap-model';

export function SitemapFindings({rows}:{rows:SitemapFinding[]}){
 const [filter,setFilter]=useState('All'),[search,setSearch]=useState(''),[issuesOnly,setIssuesOnly]=useState(false),[page,setPage]=useState(0),[exportPath,setExportPath]=useState(''),[notice,setNotice]=useState(''),[error,setError]=useState('');
 const filtered=useMemo(()=>selectSitemapFindings(rows,filter,search,issuesOnly),[rows,filter,search,issuesOnly]);
 const last=Math.max(0,Math.ceil(filtered.length/SITEMAP_PAGE_SIZE)-1),current=Math.min(page,last),visible=filtered.slice(current*SITEMAP_PAGE_SIZE,(current+1)*SITEMAP_PAGE_SIZE);
 async function save(){try{await invoke('export_table',{path:exportPath,headers:['Address','HTTP status','Membership','Findings','Address (Original)'],rows:filtered.map(row=>[displayUrl(row.url),String(row.status),row.membership,row.issues,row.url])});setNotice('Saved: '+exportPath);setError('')}catch(error){setError(String(error))}}
 return <section aria-label="Sitemap findings"><div className="filterbar">
  <label>Sitemap filter<select aria-label="Sitemap filter" value={filter} onChange={event=>{setFilter(event.target.value);setPage(0)}}>{['All','In sitemap','Missing from sitemap','Not crawled','Duplicate locations'].map(value=><option key={value}>{value}</option>)}</select></label>
  <input aria-label="Search sitemap findings" placeholder="Search URLs, status and findings…" value={search} onChange={event=>{setSearch(event.target.value);setPage(0)}}/>
  <label><input type="checkbox" checked={issuesOnly} onChange={event=>{setIssuesOnly(event.target.checked);setPage(0)}}/>Only findings</label>
 </div><p role="status">{filtered.length.toLocaleString()} matching URLs · {filtered.length?current*SITEMAP_PAGE_SIZE+1:0}–{Math.min((current+1)*SITEMAP_PAGE_SIZE,filtered.length)} shown. Export includes all matching rows.</p>
 <p>Membership refers only to the supplied sitemap inventory. Not crawled is unknown, not an HTTP error.</p>
 <div style={{maxHeight:'55vh',overflow:'auto'}}><table aria-label="Sitemap findings"><thead><tr>{['Address','HTTP status','Membership','Findings'].map(value=><th key={value}>{value}</th>)}</tr></thead><tbody>{visible.map(row=><tr key={row.url}><td dir="ltr">{displayUrl(row.url)}</td><td>{row.status}</td><td>{row.membership}</td><td>{row.issues||'No issues found in crawl data'}</td></tr>)}</tbody></table></div>
 <div className="filterbar"><button disabled={current===0} onClick={()=>setPage(0)}>First sitemap page</button><button disabled={current===0} onClick={()=>setPage(current-1)}>Previous sitemap page</button><span>Page {current+1} / {last+1}</span><button disabled={current===last} onClick={()=>setPage(current+1)}>Next sitemap page</button><button disabled={current===last} onClick={()=>setPage(last)}>Last sitemap page</button></div>
 <label>Sitemap findings CSV path<input value={exportPath} dir="ltr" onChange={event=>setExportPath(event.target.value)}/></label><NativeFileBrowse kind="export_csv" value={exportPath} onChoose={setExportPath}/><button disabled={!exportPath.trim()} onClick={()=>void save()}>Export sitemap findings</button>{notice&&<p role="status">{notice}</p>}{error&&<p role="alert">{error}</p>}
 </section>;
}

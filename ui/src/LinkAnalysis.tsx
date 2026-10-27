import {NativeFileBrowse} from './NativeFileBrowse';
import React,{useState,useEffect,useMemo} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {displayUrl,type PageRecord} from './model';
import {selectAnalysisRows,analysisCells,analysisHeaders,type LinkAnalysisReport} from './link-analysis-model';
export function LinkAnalysis({running,onNavigate,pages}:{running:boolean;onNavigate:(url:string)=>void;pages:PageRecord[]}){
 const [report,setReport]=useState<LinkAnalysisReport|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[candidates,setCandidates]=useState(false),[search,setSearch]=useState(''),[path,setPath]=useState(''),[notice,setNotice]=useState('');
 async function load(){setBusy(true);setError('');try{setReport(await invoke<LinkAnalysisReport>('analyze_link_graph'))}catch(error){setError(String(error))}finally{setBusy(false)}}
 useEffect(()=>{if(!running)void load()},[running]);
 const [pageIndex,setPageIndex]=useState(0);
 const rows=useMemo(()=>selectAnalysisRows(report?.rows??[],candidates,search),[report,candidates,search]);
 const knownUrls=useMemo(()=>new Set(pages.map(page=>page.url)),[pages]);
 useEffect(()=>setPageIndex(0),[report,candidates,search]);
 const pageCount=Math.max(1,Math.ceil(rows.length/200)),visibleRows=rows.slice(pageIndex*200,(pageIndex+1)*200);
 async function save(){try{await invoke('export_table',{path,headers:analysisHeaders,rows:rows.map(analysisCells)});setNotice('Saved: '+path);setError('')}catch(error){setError(String(error))}}
 return <section aria-label="Link scoring and orphan candidates">
  {running?<p>Stop the crawl before analyzing the saved graph.</p>:<button disabled={busy} onClick={()=>void load()}>Recalculate link analysis</button>}
  {busy&&<p role="status">Calculating link graph…</p>}{error&&<p role="alert">{error}</p>}
  {report&&<><p>{report.scored_pages} scored pages · {report.rows.filter(row=>row.orphan_candidate).length} orphan candidates · {report.iterations} iterations · {report.converged?'converged':'iteration limit reached'}</p><p>{report.note}</p>
   <label><input type="checkbox" checked={candidates} onChange={event=>setCandidates(event.target.checked)}/>Only orphan candidates</label>
   <label>Search link analysis<input dir="ltr" value={search} onChange={event=>setSearch(event.target.value)}/></label>
   <div style={{maxHeight:'50vh',overflow:'auto'}}><table><thead><tr>{analysisHeaders.slice(0,-1).map(header=><th key={header}>{header}</th>)}</tr></thead><tbody>{visibleRows.map(row=><tr key={row.url}>{analysisCells(row).slice(0,-1).map((cell,index)=><td key={index} dir={index===0?'ltr':'auto'}>{index===0?(knownUrls.has(row.url)?<button onClick={()=>onNavigate(row.url)}>{displayUrl(cell)}</button>:displayUrl(cell)):index===9&&row.score==null?'Unknown':cell}</td>)}</tr>)}</tbody></table></div>
   <div className="link-pagination"><span>{rows.length} results · page {pageIndex+1} of {pageCount}</span><button disabled={pageIndex===0} onClick={()=>setPageIndex(index=>index-1)}>Previous results</button><button disabled={pageIndex+1>=pageCount} onClick={()=>setPageIndex(index=>index+1)}>Next results</button></div>
   <label>Link analysis CSV path<input dir="ltr" value={path} onChange={event=>setPath(event.target.value)}/></label><NativeFileBrowse kind="export_csv" value={path} onChoose={setPath}/><button disabled={!path||busy} onClick={()=>void save()}>Export link analysis</button>{notice&&<p role="status">{notice}</p>}
  </>}
 </section>
}

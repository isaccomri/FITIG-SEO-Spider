import {NativeFileBrowse} from './NativeFileBrowse';
import React,{useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {displayUrl} from './model';
import {defaultTextOptions,textCells,textHeaders,type TextOptions,type TextReport} from './text-analysis-model';
export function TextAnalysis({kind,running,onNavigate}:{kind:'ngrams'|'duplicates';running:boolean;onNavigate:(url:string)=>void}){
 const [options,setOptions]=useState<TextOptions>(structuredClone(defaultTextOptions)),[report,setReport]=useState<TextReport|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[path,setPath]=useState(''),[notice,setNotice]=useState(''),[page,setPage]=useState(0);
 async function analyze(){setBusy(true);setReport(null);setError('');setNotice('');setPage(0);try{setReport(await invoke<TextReport>('analyze_saved_text',{kind,options}))}catch(error){setError(String(error))}finally{setBusy(false)}}
 useEffect(()=>{if(!running)void analyze()},[]);
 function number(name:keyof TextOptions,value:string){setOptions(options=>({...options,[name]:Number(value)}))}
 async function save(){try{await invoke('export_table',{path,headers:textHeaders(kind),rows:report?.rows.map(row=>textCells(kind,row))??[]});setNotice('Saved: '+path);setError('')}catch(error){setError(String(error))}}
 const visible=report?.rows.slice(page*200,(page+1)*200)??[],headers=textHeaders(kind).slice(0,kind==='ngrams'?3:5);
 return <section aria-label={kind==='ngrams'?'N-gram analysis':'Near-duplicate analysis'}>
  <div className="text-analysis-options">
   <label>Text area<select aria-label="Text area" value={options.scope} onChange={event=>setOptions(options=>({...options,scope:event.target.value as TextOptions['scope']}))}><option value="content">Content (exclude nav and footer)</option><option value="body">Complete body text</option></select></label>
   <label>Maximum analyzed pages (0 = all matching)<input aria-label="Maximum analyzed pages" type="number" min="0" max="1000000" value={options.max_pages} onChange={event=>number('max_pages',event.target.value)}/></label>
   <label>Maximum report rows<input type="number" min="1" max="10000" value={options.limit} onChange={event=>number('limit',event.target.value)}/></label>
   <label>URL include regex (optional)<input dir="ltr" value={options.include_pattern} onChange={event=>setOptions(options=>({...options,include_pattern:event.target.value}))}/></label>
   {kind==='ngrams'?<><label>N-gram size<input type="number" min="1" max="6" value={options.n} onChange={event=>number('n',event.target.value)}/></label><label>Minimum occurrences<input type="number" min="1" value={options.min_frequency} onChange={event=>number('min_frequency',event.target.value)}/></label><label>Minimum documents<input type="number" min="1" value={options.min_documents} onChange={event=>number('min_documents',event.target.value)}/></label><label>Stop words (one per line)<textarea rows={2} value={options.stop_words.join('\n')} onChange={event=>setOptions(options=>({...options,stop_words:event.target.value.split(/\r?\n/).map(word=>word.trim()).filter(Boolean)}))}/></label></>:<><label>Minimum similarity (%)<input type="number" min="1" max="100" value={options.threshold*100} onChange={event=>setOptions(options=>({...options,threshold:Number(event.target.value)/100}))}/></label><label>Words per shingle<input type="number" min="1" max="10" value={options.shingle_size} onChange={event=>number('shingle_size',event.target.value)}/></label></>}
  </div>
  <button disabled={running||busy} onClick={()=>void analyze()}>Analyze saved text</button>{running&&<p>Stop the crawl before analyzing saved text.</p>}{busy&&<p role="status">Analyzing stored HTML…</p>}{error&&<p role="alert">{error}</p>}
  {report&&<><p>{report.summary.processed_pages} pages analyzed · {report.matching_phrases??report.matching_pairs??0} matching {kind==='ngrams'?'phrases':'pairs'}{report.truncated?' · result row limit applied':''}</p><p>{report.summary.note}</p>{report.method&&<p>{report.method}</p>}
   {report.summary.missing_sources.length>0&&<details><summary>{report.summary.missing_sources.length} pages have no stored source</summary><ul>{report.summary.missing_sources.map(url=><li key={url}>{displayUrl(url)}</li>)}</ul></details>}
   {report.summary.insufficient_text.length>0&&<details><summary>{report.summary.insufficient_text.length} pages have insufficient words for shingles</summary><ul>{report.summary.insufficient_text.map(url=><li key={url}>{displayUrl(url)}</li>)}</ul></details>}
   <div className="text-analysis-results"><table><thead><tr>{headers.map(header=><th key={header}>{header}</th>)}</tr></thead><tbody>{visible.map((row,index)=><tr key={index}>{textCells(kind,row).slice(0,headers.length).map((cell,column)=><td key={column} dir="auto">{kind==='duplicates'&&column<2?<button onClick={()=>onNavigate(column===0?('left' in row?row.left:''):('right'in row?row.right:''))}>{cell}</button>:cell}</td>)}</tr>)}</tbody></table></div>
   <div className="link-pagination"><span>{report.rows.length} result rows · page {page+1} of {Math.max(1,Math.ceil(report.rows.length/200))}</span><button disabled={page===0} onClick={()=>setPage(page=>page-1)}>Previous text results</button><button disabled={(page+1)*200>=report.rows.length} onClick={()=>setPage(page=>page+1)}>Next text results</button></div>
   <label>Text analysis CSV path<input dir="ltr" value={path} onChange={event=>setPath(event.target.value)}/></label><NativeFileBrowse kind="export_csv" value={path} onChoose={setPath}/><button disabled={!path||busy} onClick={()=>void save()}>Export text analysis</button>{notice&&<p role="status">{notice}</p>}
  </>}
 </section>
}

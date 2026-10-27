import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {displayUrl,type PageRecord} from './model';
import {NativeFileBrowse} from './NativeFileBrowse';
type Finding={severity:string;code:string;line:number;col:number;message:string;spec_url:string};
type Report={status:string;total:number;truncated:boolean;findings:Finding[]};
export function AmpAudit({pages,selectedUrl,running}:{pages:PageRecord[];selectedUrl:string;running:boolean}){
 const [url,setUrl]=useState(selectedUrl),[report,setReport]=useState<Report|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[path,setPath]=useState(''),[notice,setNotice]=useState('');
 const alive=useRef(true);useEffect(()=>()=>{alive.current=false},[]);
 async function validate(){setBusy(true);setReport(null);setError('');setNotice('');try{const value=await invoke<Report>('validate_amp',{url});if(alive.current)setReport(value)}catch(e){if(alive.current)setError(String(e))}finally{if(alive.current)setBusy(false)}}
 async function save(){setError('');try{await invoke('export_table',{path,headers:['Address','Status','Severity','Code','Line','Column','Message','Specification'],rows:report?.findings.map(f=>[url,report.status,f.severity,f.code,String(f.line),String(f.col),f.message,f.spec_url])??[]});setNotice('Saved: '+path)}catch(e){setError(String(e))}}
 return <section aria-label="AMP validation"><p>Official AMP HTML validator, pinned offline snapshot. Validates recorded HTTP source for the selected URL; scripts in that source are not executed. This checks AMP syntax, not search indexing or discoverability. Maximum source: 2 MiB.</p>
 <label>AMP URL<select aria-label="AMP URL" value={url} disabled={busy} onChange={e=>{setUrl(e.target.value);setReport(null);setError('')}}>{!pages.some(p=>p.url===url)&&<option value={url}>{displayUrl(url)}</option>}{pages.map(p=><option key={p.url} value={p.url}>{displayUrl(p.url)}</option>)}</select></label><button disabled={running||busy||!url} onClick={()=>void validate()}>Validate recorded AMP source</button>
 {running&&<p>Stop the crawl before validation.</p>}{busy&&<p role="status">Validating AMP locally…</p>}{error&&<p role="alert">{error}</p>}
 {report&&<><p role="status">AMP {report.status} · {report.total} findings</p>{report.truncated&&<p>Details truncated to 80 rows / 24 KiB; total includes omitted findings.</p>}<div className="text-analysis-results"><table aria-label="AMP findings"><thead><tr>{['Severity','Code','Line','Column','Message','Specification'].map(h=><th key={h}>{h}</th>)}</tr></thead><tbody>{report.findings.map((f,i)=><tr key={i}><td>{f.severity}</td><td>{f.code}</td><td>{f.line}</td><td>{f.col}</td><td dir="auto">{f.message}</td><td>{f.spec_url}</td></tr>)}</tbody></table></div><label>AMP CSV path<input value={path} dir="ltr" onChange={e=>setPath(e.target.value)}/></label><NativeFileBrowse kind="export_csv" value={path} onChoose={setPath}/><button disabled={!path||busy} onClick={()=>void save()}>Export AMP findings</button>{notice&&<p role="status">{notice}</p>}</>}
 </section>
}

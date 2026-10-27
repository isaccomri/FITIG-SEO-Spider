import React from 'react';
import {displayUrl} from './model';
export function DataView({value}:{value:unknown}){
 if(value==null)return <span>—</span>;
 if(typeof value!=='object')return <span dir="auto">{displayUrl(String(value))}</span>;
 if(Array.isArray(value)){if(!value.length)return <p className="muted">No entries.</p>;if(value.every(v=>v&&typeof v==='object'&&!Array.isArray(v))){const keys=[...new Set(value.flatMap(v=>Object.keys(v)))];return <div className="data-scroll"><table><thead><tr>{keys.map(k=><th key={k}>{k.replaceAll('_',' ')}</th>)}</tr></thead><tbody>{value.map((row,i)=><tr key={i}>{keys.map(k=><td key={k}><DataView value={row[k]}/></td>)}</tr>)}</tbody></table></div>}return <ul>{value.map((v,i)=><li key={i}><DataView value={v}/></li>)}</ul>}
 return <dl className="data-properties">{Object.entries(value).map(([key,v])=><React.Fragment key={key}><dt>{displayUrl(key.replaceAll('_',' '))}</dt><dd><DataView value={v}/></dd></React.Fragment>)}</dl>
}

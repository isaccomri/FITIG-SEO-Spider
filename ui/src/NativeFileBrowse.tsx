import React,{useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
export function NativeFileBrowse({kind,value,onChoose,disabled=false}:{kind:string;value:string;onChoose:(value:string)=>void;disabled?:boolean}){
 const [busy,setBusy]=useState(false),[error,setError]=useState(''),active=useRef(true);
 useEffect(()=>{active.current=true;return()=>{active.current=false}},[]);
 async function browse(){setBusy(true);setError('');try{const selected=await invoke<string|null>('choose_file_path',{kind,initial:value});if(active.current&&selected!==null)onChoose(selected)}catch(error){if(active.current)setError(String(error))}finally{if(active.current)setBusy(false)}}
 return <><button type="button" disabled={busy||disabled} onClick={()=>void browse()}>{busy?'Choosing file…':'Browse…'}</button>{error&&<p role="alert">{error}</p>}</>;
}

import {useRef,useState,useEffect,useCallback} from 'react';
import {resultWindow,RESULT_ROW_HEIGHT,RESULT_HEADER_HEIGHT} from './results-window-model';
export function useResultsWindow(count:number,resetKey:string){
 const ref=useRef<HTMLDivElement>(null),[geometry,setGeometry]=useState({top:0,height:360});
 const focusRequest=useRef(0);
 useEffect(()=>()=>{focusRequest.current++},[]);
 const update=useCallback(()=>{const element=ref.current;if(element)setGeometry(previous=>previous.top===element.scrollTop&&previous.height===element.clientHeight?previous:{top:element.scrollTop,height:element.clientHeight})},[]);
 useEffect(()=>{const element=ref.current;if(!element)return;update();const observer=new ResizeObserver(update);observer.observe(element);return()=>observer.disconnect()},[update]);
 useEffect(()=>{if(ref.current)ref.current.scrollTop=0;update()},[resetKey,update]);
 const focusIndex=useCallback((index:number)=>{const element=ref.current;if(!element)return;const request=++focusRequest.current;const top=RESULT_HEADER_HEIGHT+index*RESULT_ROW_HEIGHT,bottom=top+RESULT_ROW_HEIGHT;if(top<element.scrollTop+RESULT_HEADER_HEIGHT)element.scrollTop=index*RESULT_ROW_HEIGHT;else if(bottom>element.scrollTop+element.clientHeight)element.scrollTop=bottom-element.clientHeight;update();requestAnimationFrame(()=>requestAnimationFrame(()=>{
  // A newer navigation request supersedes this one. Never pull focus back from another control.
  // Body focus is allowed because scrolling may have unmounted the previously focused virtual row.
  if(request!==focusRequest.current||ref.current!==element||!element.isConnected)return;
  const active=document.activeElement;if(active!==document.body&&!element.contains(active))return;
  element.querySelector<HTMLElement>(`tr[data-row-index="${index}"]`)?.focus({preventScroll:true});
 }));},[update]);
 return {ref,onScroll:update,range:resultWindow(count,geometry.top,geometry.height),focusIndex,pageSize:Math.max(1,Math.floor(geometry.height/RESULT_ROW_HEIGHT)-1)};
}

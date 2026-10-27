import {categories,columnsFor} from './model';
export const WORKBENCH_PREFERENCES_KEY='fitig.workbench.v1';
export type WorkbenchPreferences={version:1;dark:boolean;hebrew:boolean;details_ratio:number;overview_width:number;columns:Record<string,string[]>;widths:Record<string,Record<string,number>>};
export const defaultPreferences=():WorkbenchPreferences=>({version:1,dark:true,hebrew:false,details_ratio:.35,overview_width:255,columns:{},widths:{}});
export function clamp(value:number,min:number,max:number,fallback:number){return Number.isFinite(value)?Math.min(max,Math.max(min,value)):fallback}
export function parsePreferences(raw:string|null):WorkbenchPreferences{
 const result=defaultPreferences();if(!raw||raw.length>65536)return result;
 try{const value=JSON.parse(raw);if(!value||value.version!==1)return result;if(typeof value.dark==='boolean')result.dark=value.dark;if(typeof value.hebrew==='boolean')result.hebrew=value.hebrew;result.details_ratio=clamp(value.details_ratio,.15,.7,.35);result.overview_width=Math.round(clamp(value.overview_width,180,480,255));
  for(const category of categories){const defaults=columnsFor(category);const saved=value.columns?.[category];if(Array.isArray(saved)){const order=saved.filter((key:unknown):key is string=>typeof key==='string'&&defaults.includes(key));result.columns[category]=[...new Set([...order,...defaults])]}
   const widths=value.widths?.[category];if(widths&&typeof widths==='object'&&!Array.isArray(widths)){for(const key of defaults){if(typeof widths[key]==='number'&&Number.isFinite(widths[key])){result.widths[category]??={};result.widths[category][key]=Math.round(clamp(widths[key],60,1400,180));}}}
  }
 }catch{}return result;
}
export function orderedColumns(preferences:WorkbenchPreferences,category:string){const defaults=columnsFor(category);return [...new Set([...(preferences.columns[category]??[]).filter(key=>defaults.includes(key)),...defaults])]}
export function defaultColumnWidth(key:string){return key==='url'?420:key==='title'?280:key==='description'?400:key==='content_type'?230:key==='status'?110:key==='depth'?90:180}
export function columnWidth(preferences:WorkbenchPreferences,category:string,key:string){return preferences.widths[category]?.[key]??defaultColumnWidth(key)}
export function reorderColumns(columns:string[],from:string,to:string){if(!columns.includes(from)||!columns.includes(to)||from===to)return columns;const next=columns.filter(key=>key!==from);next.splice(next.indexOf(to),0,from);return next}
export function serializePreferences(preferences:WorkbenchPreferences){return JSON.stringify(parsePreferences(JSON.stringify(preferences)))}

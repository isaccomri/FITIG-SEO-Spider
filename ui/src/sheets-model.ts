import {getCell,type PageRecord} from './model';
export type SheetInput={title:string;headers:string[];rows:(string|number|boolean|null)[][];right_to_left:boolean};
const numbers=new Set(['status','depth','word_count','response_time_ms','size_bytes','url_length','title_length','description_length','image_count','link_count','json_ld_count','hreflang_count','unique_inlinks','unique_internal_outlinks','unique_external_outlinks','image_inlinks','source_count']);
export function sheetInput(title:string,columns:string[],pages:PageRecord[],labels:Record<string,string>,rtl:boolean):SheetInput{
 if(!pages.length)throw Error('Choose a result table with at least one matching URL');
 if(columns.length+1>64||(pages.length+1)*(columns.length+1)>10000)throw Error('At most 10,000 cells including headers are supported; narrow the filter or columns');
 return {title,headers:[...columns.map(key=>labels[key]??key),'Address (Original)'],rows:pages.map(page=>[...columns.map(key=>{const value=getCell(page,key);if(numbers.has(key)){if(value==='')return null;const n=Number(value);if(Number.isFinite(n)&&(!Number.isInteger(n)||Number.isSafeInteger(n)))return n}return String(value)}),page.url]),right_to_left:rtl};
}

import {displayUrl} from './model';
export type TextOptions={n:number;limit:number;min_frequency:number;min_documents:number;scope:'content'|'body';max_pages:number;include_pattern:string;stop_words:string[];threshold:number;shingle_size:number};
export const defaultTextOptions:TextOptions={n:2,limit:1000,min_frequency:2,min_documents:1,scope:'content',max_pages:0,include_pattern:'',stop_words:[],threshold:.9,shingle_size:5};
export type TextSummary={processed_pages:number;missing_sources:string[];insufficient_text:string[];note:string};
export type NgramRow={phrase:string;occurrences:number;documents:number};
export type DuplicateRow={left:string;right:string;similarity:number;shared_shingles:number;union_shingles:number};
export type TextReport={summary:TextSummary;rows:(NgramRow|DuplicateRow)[];matching_phrases?:number;matching_pairs?:number;truncated:boolean;method?:string};
export function textHeaders(kind:'ngrams'|'duplicates'){return kind==='ngrams'?['Phrase','Occurrences','Documents']:['From','To','Similarity %','Shared Shingles','Union Shingles','From (Original)','To (Original)']}
export function textCells(kind:'ngrams'|'duplicates',value:NgramRow|DuplicateRow){if(kind==='ngrams'){const row=value as NgramRow;return [row.phrase,String(row.occurrences),String(row.documents)]}const row=value as DuplicateRow;return [displayUrl(row.left),displayUrl(row.right),(row.similarity*100).toFixed(4),String(row.shared_shingles),String(row.union_shingles),row.left,row.right]}

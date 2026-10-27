import {type PageRecord} from './model';
export type WordTerm={word:string;count:number};
const common=new Set('the and for with this that from your you are was were have has not but our can all into its this about a an of to in on at is it as or be by we i של את על עם לא זה זו הוא היא הם הן גם או כל יש אין אני אתה אשר כי אם מה אל עד בין מן שלוש'.split(' '));
export function tokenizeWords(text:string){return (text.toLocaleLowerCase().match(/[\p{L}\p{N}]+/gu)??[]).filter(word=>[...word].length>=2)}
export function wordTerms(pages:PageRecord[],url:string,source:'anchor'|'body',hideCommon=true):{terms:WordTerm[];recorded:boolean}{
 const count=new Map<string,number>();const add=(text:string)=>{for(const word of tokenizeWords(text))count.set(word,(count.get(word)??0)+1)};
 if(source==='anchor'){let target:URL;try{target=new URL(url)}catch{return {terms:[],recorded:true}}for(const p of pages){try{if(typeof p.internal==='boolean'?!p.internal:new URL(p.url).origin!==target.origin)continue}catch{continue}for(const link of p.links??[])if(link.url===url&&(!link.kind||link.kind==='anchor'))add((link.text??'')+' '+(link.alt??''))}}
 else{const p=pages.find(p=>p.url===url);if(p?.body_terms==null)return {terms:[],recorded:false};for(const [word,n] of Object.entries(p.body_terms as Record<string,number>))if(Number.isFinite(n)&&n>0)count.set(word,n)}
 return {terms:[...count].filter(([word])=>!hideCommon||!common.has(word)).map(([word,count])=>({word,count})).sort((a,b)=>b.count-a.count||a.word.localeCompare(b.word)),recorded:true};
}
// Deterministic shelf layout: bounded fonts, explicit rows, no word collisions.
export function wordCloudLayout(terms:WordTerm[]){const shown=terms.slice(0,80),top=shown[0]?.count??1;let x=30,y=55,rowHeight=0;const words=shown.map(term=>{const size=14+38*Math.sqrt(term.count/top),width=Math.min(900,[...term.word].length*size*.73+20);if(x+width>970){x=30;y+=rowHeight+18;rowHeight=0}const result={...term,x,y,size};x+=width;rowHeight=Math.max(rowHeight,size);return result});return {words,height:Math.max(400,y+rowHeight+30)}}

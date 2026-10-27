export const RESULT_ROW_HEIGHT=30,RESULT_HEADER_HEIGHT=33;
export function resultWindow(count:number,scrollTop:number,height:number,overscan=10){
 count=Math.max(0,Math.floor(Number.isFinite(count)?count:0));scrollTop=Number.isFinite(scrollTop)?Math.max(0,scrollTop):0;height=Number.isFinite(height)?Math.max(0,height):360;
 if(!count)return {start:0,end:0,top:0,bottom:0};
 const first=Math.min(count-1,Math.floor(Math.max(0,scrollTop-RESULT_HEADER_HEIGHT)/RESULT_ROW_HEIGHT)),visible=Math.max(1,Math.ceil(height/RESULT_ROW_HEIGHT));const start=Math.max(0,first-overscan),end=Math.min(count,first+visible+overscan+1);
 return {start,end,top:start*RESULT_ROW_HEIGHT,bottom:(count-end)*RESULT_ROW_HEIGHT};
}

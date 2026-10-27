// The caller supplies saved source as a JSON string, never as executable HTML.
(async function(source){
 await amp.validator.init();
 const result=amp.validator.validateString(source,'AMP');
 const findings=[];let bytes=0;
 for(const error of result.errors){
  const row={severity:error.severity,code:error.code,line:error.line,col:error.col,message:amp.validator.renderErrorMessage(error).slice(0,1000),spec_url:(error.specUrl||'').slice(0,500)};
  const size=new TextEncoder().encode(JSON.stringify(row)).length;
  if(findings.length>=80||bytes+size>24000)break;
  findings.push(row);bytes+=size;
 }
 return {status:result.status,total:result.errors.length,truncated:findings.length<result.errors.length,findings};
})

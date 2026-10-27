if(document.querySelectorAll('*').length>20000)throw new Error('Accessibility audit exceeds the 20,000-element page limit');
const result=await window.axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa','best-practice']},iframes:false,preload:false,resultTypes:['violations','incomplete']});
const report={version:window.axe.version,url:location.href,counts:{violations:result.violations.length,incomplete:result.incomplete.length,passes:result.passes.length,inapplicable:result.inapplicable.length,violation_nodes:result.violations.reduce((sum,rule)=>sum+rule.nodes.length,0),incomplete_nodes:result.incomplete.reduce((sum,rule)=>sum+rule.nodes.length,0)},rows:[],error:'',truncated:false};
const text=(value,length)=>{const raw=String(value??'');if(raw.length>length)report.truncated=true;return raw.slice(0,length)};
for(const [key,status]of [['violations','Violation'],['incomplete','Needs review']])for(const rule of result[key])for(const node of rule.nodes){
 if(rule.tags.length>16)report.truncated=true;
 const row={rule:text(rule.id,128),status,impact:text(node.impact??rule.impact,32),help:text(rule.help,240),help_url:text(rule.helpUrl,512),tags:rule.tags.slice(0,16).map(tag=>text(tag,64)),selector:text(JSON.stringify(node.target),1024),html:text(node.html,512),summary:text(node.failureSummary,1600)};
 report.rows.push(row);
 if(report.rows.length>80||new TextEncoder().encode(JSON.stringify(report)).length>28000){report.rows.pop();report.truncated=true;}
}
return report;

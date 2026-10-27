const path=require('path'),fs=require('fs'),{spawn}=require('child_process');
async function action(processId,mode,target){return new Promise((resolve,reject)=>{const child=spawn(process.env.FITIG_PYTHON||'python',['-X','utf8',path.join(__dirname,'own-file-dialog.py'),String(processId),mode,...(target?[target]:[])],{windowsHide:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='';child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);child.on('error',reject);child.on('close',code=>code===0?resolve(JSON.parse(stdout)):reject(Error(stderr||stdout)));})}
async function choose({scope,processId,target,input,button}){
 const root=path.resolve(process.env.FITIG_ACCEPTANCE_DIR),resolved=path.resolve(target),relative=path.relative(root,resolved);
 if(!processId||relative.startsWith('..')||path.isAbsolute(relative))throw Error('Native acceptance may select only its owned output files');
 const existed=fs.existsSync(resolved);await (button||scope.getByRole('button',{name:'Browse…',exact:true})).click();await action(processId,'select',resolved);if(existed)await action(processId,'overwrite-yes');
 await input.evaluate((element,value)=>new Promise((resolve,reject)=>{let tries=0;const timer=setInterval(()=>{if(element.value===value){clearInterval(timer);resolve(true)}else if(++tries>100){clearInterval(timer);reject(Error('Selected native path not reflected in input'))}},50)}),resolved);
}
module.exports={choose};

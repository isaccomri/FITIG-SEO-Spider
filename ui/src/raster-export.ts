export function rasterSize(width:number,height:number,scale:number){
 if(![width,height,scale].every(Number.isFinite)||width<=0||height<=0||![1,2].includes(scale))throw Error('PNG export requires positive finite SVG dimensions and scale 1 or 2.');
 const factor=Math.min(scale,4096/width,4096/height);
 return {width:Math.max(1,Math.round(width*factor)),height:Math.max(1,Math.round(height*factor)),limited:factor<scale};
}
export function svgDocument(source:string){
 if(source.length>32*1024*1024||/<!DOCTYPE|<!ENTITY/i.test(source))throw Error('Invalid or oversized chart SVG.');
 const document=new DOMParser().parseFromString(source,'image/svg+xml'),root=document.documentElement;
 if(document.querySelector('parsererror')||root.localName!=='svg'||root.namespaceURI!=='http://www.w3.org/2000/svg')throw Error('Expected chart SVG.');
 // Export only self-contained chart primitives; do not load resources from a chart label or embedded document.
 const resourceStyles=new Set(['style','fill','stroke','filter','clip-path','mask','cursor','font','font-family']);
 for(const element of [root,...Array.from(root.querySelectorAll('*'))]){if(['script','foreignObject','image','use','style','iframe','animate','set'].includes(element.localName))throw Error('PNG export accepts self-contained chart primitives only.');for(const attribute of Array.from(element.attributes))if(/^on/i.test(attribute.name)||/href$/i.test(attribute.name)||resourceStyles.has(attribute.name)&&/url\s*\(|@import/i.test(attribute.value))throw Error('PNG export cannot load external resources.');}
 const box=(root.getAttribute('viewBox')??'').trim().split(/[\s,]+/).map(Number);
 if(box.length!==4||!box.every(Number.isFinite)||box[2]<=0||box[3]<=0)throw Error('Chart requires a finite positive viewBox.');
 return {root:root as unknown as SVGSVGElement,width:box[2],height:box[3]};
}
export async function rasterizeSvg(source:string,scale:number):Promise<string>{
 const {root,width,height}=svgDocument(source),size=rasterSize(width,height,scale);
 root.setAttribute('width',String(size.width));root.setAttribute('height',String(size.height));
 root.style.width=size.width+'px';root.style.height=size.height+'px';root.style.minWidth='0';root.style.minHeight='0';
 const background=root.style.backgroundColor||root.style.background||'#ffffff';
 await document.fonts.ready;
 const url='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(root));
 {
  const image=new Image();await new Promise<void>((resolve,reject)=>{const timer=setTimeout(()=>{image.src='';reject(Error('Chart image decoding timed out.'));},15000);image.onload=()=>{clearTimeout(timer);resolve()};image.onerror=()=>{clearTimeout(timer);reject(Error('Unable to decode chart SVG.'))};image.src=url;});
  const canvas=document.createElement('canvas');canvas.width=size.width;canvas.height=size.height;const context=canvas.getContext('2d');if(!context)throw Error('PNG canvas is unavailable.');context.fillStyle=background;context.fillRect(0,0,size.width,size.height);context.drawImage(image,0,0,size.width,size.height);
  const encoded=canvas.toDataURL('image/png');if(!encoded.startsWith('data:image/png;base64,')||encoded.length>Math.ceil(8*1024*1024/3)*4+22)throw Error('Chart PNG exceeds the 8 MiB export limit.');return encoded;
 }
}

import {displayUrl,status,type PageRecord} from './model';
export type GraphLayout='tree'|'force'|'force3d';
export type GraphStructure='crawl'|'directory';
export type GraphNode={url:string;label:string;shortLabel:string;status:number;x:number;y:number;z:number;depth:number;synthetic:boolean;indexable:boolean;inlinks:number};
export type GraphEdge={from:string;to:string};
export type GraphOptions={structure?:GraphStructure;seed?:string;resources?:boolean;external?:boolean;focus?:string;collapsed?:string[]};
export type CrawlGraph={nodes:GraphNode[];edges:GraphEdge[];allEdges:GraphEdge[];total:number;truncated:boolean;width:number;height:number;roots:string[];excluded:number;matched:number};
function origin(url:string){try{return new URL(url).origin}catch{return ''}}
function normal(url:string){try{const u=new URL(url);u.hash='';return u.href}catch{return url}}
function shortLabel(url:string){try{const u=new URL(url);return displayUrl(u.pathname==='/'&&!u.search?u.host:u.pathname+u.search)}catch{return displayUrl(url)}}
function isHtml(p:PageRecord){return /html/i.test(String(p.content_type??''))||(!p.content_type&&!/\.(?:png|jpe?g|gif|webp|svg|ico|css|js|pdf|woff2?|ttf|xml|zip)(?:\?|$)/i.test(p.url))}
export function graphIndexable(p:PageRecord){return status(p)>=200&&status(p)<300&&!/(?:^|[\s,])(noindex|none)(?:$|[\s,])/i.test(p.directives??'')&&(!p.canonical||normal(p.canonical)===normal(p.url))}
function node(p:PageRecord):GraphNode{return {url:p.url,label:displayUrl(p.url),shortLabel:shortLabel(p.url),status:status(p),x:0,y:0,z:0,depth:0,synthetic:false,indexable:graphIndexable(p),inlinks:0}}
function forest(nodes:GraphNode[],edges:GraphEdge[],seed:string){
 const adjacency=new Map(nodes.map(n=>[n.url,[] as string[]]));for(const e of edges)adjacency.get(e.from)?.push(e.to);
 const seen=new Set<string>(),tree:GraphEdge[]=[],roots:string[]=[];const byId=new Map(nodes.map(n=>[n.url,n]));
 for(const root of [...(byId.has(seed)?[byId.get(seed)!]:[]),...nodes]){if(seen.has(root.url))continue;roots.push(root.url);seen.add(root.url);root.depth=0;const queue=[root.url];for(let i=0;i<queue.length;i++){const from=queue[i];for(const to of adjacency.get(from)??[]){if(seen.has(to)||!byId.has(to))continue;seen.add(to);byId.get(to)!.depth=byId.get(from)!.depth+1;tree.push({from,to});queue.push(to)}}}
 return {tree,roots};
}
function descendants(root:string,edges:GraphEdge[]){const children=new Map<string,string[]>();for(const e of edges){const row=children.get(e.from)??[];row.push(e.to);children.set(e.from,row)}const seen=new Set([root]),queue=[root];for(let i=0;i<queue.length;i++)for(const to of children.get(queue[i])??[])if(!seen.has(to)){seen.add(to);queue.push(to)}return seen}
function directory(pages:PageRecord[]){
 const nodes:GraphNode[]=[],edges:GraphEdge[]=[],byId=new Map<string,GraphNode>(),byUrl=new Map(pages.map(p=>[p.url,p]));
 const add=(id:string,label:string,parent?:string,actual?:PageRecord)=>{if(!byId.has(id)){const n=actual?node(actual):{...node({url:id}),synthetic:true,label,shortLabel:label};nodes.push(n);byId.set(id,n);if(parent)edges.push({from:parent,to:id})}return id};
 for(const p of pages){let u:URL;try{u=new URL(p.url)}catch{continue}const protocol='directory:'+u.protocol;add(protocol,u.protocol+'//');const root=u.origin+'/';let parent=add(root,u.host,protocol,byUrl.get(root));let path='';const parts=u.pathname.split('/').filter(Boolean);for(let i=0;i<parts.length;i++){path+='/'+parts[i];const last=i===parts.length-1;const key=last&&!u.search?p.url:u.origin+path+'/';parent=add(key,displayUrl(parts[i]),parent,byUrl.get(key))}if(u.search)parent=add(p.url,displayUrl(u.search),parent,p);if(!parts.length&&!u.search&&p.url!==root)add(p.url,shortLabel(p.url),parent,p)}
 return {nodes,edges};
}
function treePositions(nodes:GraphNode[],edges:GraphEdge[],roots:string[]){
 const children=new Map(nodes.map(n=>[n.url,[] as string[]])),byId=new Map(nodes.map(n=>[n.url,n]));for(const e of edges)children.get(e.from)?.push(e.to);
 const order:string[]=[],stack=[...roots].reverse();while(stack.length){const id=stack.pop()!;order.push(id);const row=children.get(id)??[];for(let i=row.length-1;i>=0;i--)stack.push(row[i])}
 let slot=0;for(const id of order)if(!children.get(id)?.length)byId.get(id)!.y=50+slot++*28;
 for(const id of [...order].reverse()){const n=byId.get(id)!,row=children.get(id)??[];if(row.length)n.y=(byId.get(row[0])!.y+byId.get(row[row.length-1])!.y)/2;n.x=170+n.depth*300}
 return {width:Math.max(1000,480+Math.max(0,...nodes.map(n=>n.depth))*300),height:Math.max(650,100+slot*28)};
}
function forcePositions(nodes:GraphNode[],edges:GraphEdge[],three:boolean){
 const index=new Map(nodes.map((n,i)=>[n.url,i]));nodes.forEach((n,i)=>{const r=50+18*Math.sqrt(i);n.x=r*Math.cos(i*2.399963);n.y=r*Math.sin(i*2.399963);n.z=three?160*Math.sin(i*1.713):0});
 const pairs=edges.map(e=>[index.get(e.from)!,index.get(e.to)!]);const stride=Math.max(1,Math.floor(nodes.length/300));
 for(let step=0;step<75;step++){const f=nodes.map(()=>({x:0,y:0,z:0}));for(let i=0;i<nodes.length;i++)for(let j=i+1;j<nodes.length;j+=stride){const a=nodes[i],b=nodes[j],dx=a.x-b.x,dy=a.y-b.y,dz=a.z-b.z,d2=Math.max(30,dx*dx+dy*dy+dz*dz),k=1200*stride/d2;f[i].x+=dx*k;f[i].y+=dy*k;f[i].z+=dz*k;f[j].x-=dx*k;f[j].y-=dy*k;f[j].z-=dz*k}for(const [i,j]of pairs){const a=nodes[i],b=nodes[j],dx=b.x-a.x,dy=b.y-a.y,dz=b.z-a.z,d=Math.max(1,Math.hypot(dx,dy,dz)),k=(d-80)*.04/d;f[i].x+=dx*k;f[i].y+=dy*k;f[i].z+=dz*k;f[j].x-=dx*k;f[j].y-=dy*k;f[j].z-=dz*k}nodes.forEach((n,i)=>{n.x+=Math.max(-12,Math.min(12,f[i].x-n.x*.003));n.y+=Math.max(-12,Math.min(12,f[i].y-n.y*.003));n.z+=Math.max(-12,Math.min(12,f[i].z-n.z*.003))})}
 const max=Math.max(1,...nodes.flatMap(n=>[Math.abs(n.x),Math.abs(n.y),Math.abs(n.z)]));nodes.forEach(n=>{n.x=500+n.x*290/max;n.y=325+n.y*290/max;n.z=n.z*290/max});return {width:1000,height:650};
}
export function project3d(n:GraphNode,yaw:number,pitch:number){const x=n.x-500,y=n.y-325,z=n.z,rx=x*Math.cos(yaw)+z*Math.sin(yaw),rz=-x*Math.sin(yaw)+z*Math.cos(yaw),ry=y*Math.cos(pitch)-rz*Math.sin(pitch),depth=y*Math.sin(pitch)+rz*Math.cos(pitch),scale=900/(900+depth);return {x:500+rx*scale,y:325+ry*scale,scale,depth}}
export function crawlGraph(pages:PageRecord[],layout:GraphLayout,query='',maximum=1000,options:GraphOptions={}):CrawlGraph{
 const all=[...new Map(pages.map(p=>[p.url,p])).values()];const seed=options.seed||all[0]?.url||'',baseOrigin=origin(seed);
 const eligible=all.filter(p=>(options.resources||isHtml(p))&&(options.external||!baseOrigin||(typeof p.internal==='boolean'?p.internal:origin(p.url)===baseOrigin)));
 let nodes:GraphNode[],allEdges:GraphEdge[];
 if(options.structure==='directory'){const d=directory(eligible);nodes=d.nodes;allEdges=d.edges}else{nodes=eligible.map(node);const ids=new Set(nodes.map(n=>n.url)),seen=new Set<string>();allEdges=[];for(const p of eligible){const targets=(p.links??[]).filter(l=>options.resources||!l.kind||l.kind==='anchor').map(l=>l.url);if(options.resources)targets.push(...(p.images??[]).map(([url])=>url));const redirect=String(p.redirect??p.redirect_url??'');if(redirect)targets.push(redirect);for(const to of targets){const key=JSON.stringify([p.url,to]);if(to===p.url||!ids.has(to)||seen.has(key))continue;seen.add(key);allEdges.push({from:p.url,to})}}}
 const byId=new Map(nodes.map(n=>[n.url,n]));for(const e of allEdges)byId.get(e.to)!.inlinks++;
 const full=forest(nodes,allEdges,options.structure==='directory'?'':seed),parent=new Map(full.tree.map(e=>[e.to,e.from]));let allowed=new Set(nodes.map(n=>n.url));
 if(options.focus&&byId.has(options.focus))allowed=descendants(options.focus,full.tree);
 const matches=new Set(nodes.filter(n=>allowed.has(n.url)&&(!query||n.label.toLowerCase().includes(query.toLowerCase()))).map(n=>n.url));
 if(query){const withAncestors=new Set(matches);for(const id of matches){let p=parent.get(id);while(p&&allowed.has(p)){withAncestors.add(p);p=parent.get(p)}}allowed=new Set([...allowed].filter(id=>withAncestors.has(id)))}
 for(const id of query?[]:options.collapsed??[]){if(!allowed.has(id))continue;for(const child of descendants(id,full.tree))if(child!==id)allowed.delete(child)}
 const children=new Map<string,string[]>();for(const e of full.tree){const row=children.get(e.from)??[];row.push(e.to);children.set(e.from,row)}const ordered:string[]=[],stack=[...full.roots].reverse();while(stack.length){const id=stack.pop()!;if(allowed.has(id))ordered.push(id);const row=children.get(id)??[];for(let i=row.length-1;i>=0;i--)stack.push(row[i])}
 const total=ordered.length,cap=Math.max(1,Math.min(5000,Math.floor(maximum)||1000)),ids=new Set(ordered.slice(0,cap));nodes=ordered.slice(0,cap).map(id=>byId.get(id)!);const edges=full.tree.filter(e=>ids.has(e.from)&&ids.has(e.to));allEdges=allEdges.filter(e=>ids.has(e.from)&&ids.has(e.to));const childIds=new Set(edges.map(e=>e.to)),roots=nodes.filter(n=>!childIds.has(n.url)).map(n=>n.url);
 const depth=new Map<string,number>();for(const root of roots)depth.set(root,0);for(const n of nodes){const p=parent.get(n.url);n.depth=p&&ids.has(p)?(depth.get(p)??0)+1:0;depth.set(n.url,n.depth)}
 const size=layout==='tree'?treePositions(nodes,edges,roots):forcePositions(nodes,edges,layout==='force3d');
 return {nodes,edges,allEdges,total,truncated:total>cap,...size,roots,excluded:all.length-eligible.length,matched:matches.size};
}

// Development-only visual fixture. This entry is not included in the production index.html build.
import React from 'react';import {createRoot} from 'react-dom/client';import {LinkTable} from './LinkTable';import './style.css';
const url='https://fixture.test/%D7%A9%D7%9C%D7%95%D7%9D';
const pages=[{url:'https://fixture.test/',status:200,links:[{url,kind:'anchor',text:'מוצר לדוגמה',position:'Navigation',path_type:'Root-relative',target:'_blank'},{url,kind:'anchor',text:'קישור תחתון',rel:'nofollow',position:'Footer',path_type:'Root-relative'}]},{url,status:200}];
createRoot(document.getElementById('fixture')!).render(<div className="app dark"><header className="titlebar">Link table fixture — synthetic test records, not a live crawl</header><LinkTable pages={pages} url={url} direction="Inlinks" onNavigate={()=>{}} onError={error=>console.error(error)}/></div>);

import {type GraphLayout,type GraphStructure} from './graph-model';
export type GraphViewId='force'|'crawl-tree'|'crawl-3d'|'directory-force'|'directory-tree'|'directory-3d';
export const graphViews:Record<string,{id:GraphViewId;layout:GraphLayout;structure:GraphStructure}>={
 'Force-directed crawl diagram':{id:'force',layout:'force',structure:'crawl'},
 '3D force-directed crawl diagram':{id:'crawl-3d',layout:'force3d',structure:'crawl'},
 'Crawl tree graph':{id:'crawl-tree',layout:'tree',structure:'crawl'},
 'Force-directed directory tree diagram':{id:'directory-force',layout:'force',structure:'directory'},
 '3D force-directed directory tree diagram':{id:'directory-3d',layout:'force3d',structure:'directory'},
 'Directory tree graph':{id:'directory-tree',layout:'tree',structure:'directory'}
};

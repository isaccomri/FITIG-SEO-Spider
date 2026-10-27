import {describe,it,expect} from 'vitest';
import {parseUrlList,siteTree} from './workspace-model';
describe('working workspace tools',()=>{
 it('normalizes pasted URLs, ignores empty lines and deduplicates fragments',()=>expect(parseUrlList('\ufeffhttps://e.test/a#x\n\nhttps://e.test/a#y\r\nhttps://other.test/')).toEqual(['https://e.test/a','https://other.test/']));
 it('rejects empty lists, non-HTTP protocols and reports the invalid line',()=>{expect(()=>parseUrlList(' ')).toThrow('at least one');expect(()=>parseUrlList('javascript:alert(1)')).toThrow('HTTP');expect(()=>parseUrlList('https://e.test/\nbad')).toThrow('line 2')});
 it('builds a real directory tree without losing query variants or counting duplicate records',()=>{const tree=siteTree([{url:'https://e.test/'},{url:'https://e.test/folder/a?x=1'},{url:'https://e.test/folder/a?x=2'},{url:'https://e.test/folder/a?x=2'}]);expect(tree.count).toBe(3);const host=tree.children[0];expect(host.url).toBe('https://e.test/');expect(host.children[0].label).toBe('folder');expect(host.children[0].children[0].children).toHaveLength(2)});
});

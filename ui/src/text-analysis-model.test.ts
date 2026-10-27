import {expect,it} from 'vitest';
import {textCells,textHeaders} from './text-analysis-model';
it('preserves Hebrew phrases and numeric count values in export cells',()=>{expect(textCells('ngrams',{phrase:'שלום עולם',occurrences:4,documents:2})).toEqual(['שלום עולם','4','2']);expect(textHeaders('ngrams')).toHaveLength(3)});
it('exports readable and original URLs with an explicit similarity percentage',()=>{const left='https://e.test/%D7%A9%D7%9C%D7%95%D7%9D',right='https://e.test/b';const cells=textCells('duplicates',{left,right,similarity:.9,shared_shingles:9,union_shingles:10});expect(cells[0]).toBe('https://e.test/שלום');expect(cells[2]).toBe('90.0000');expect(cells.slice(-2)).toEqual([left,right]);expect(cells).toHaveLength(textHeaders('duplicates').length)});

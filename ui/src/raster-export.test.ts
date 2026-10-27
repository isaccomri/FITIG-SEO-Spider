import {describe,it,expect} from 'vitest';
import {rasterSize} from './raster-export';
describe('PNG chart geometry',()=>{
 it('preserves graph aspect ratio at one and two times resolution',()=>{expect(rasterSize(1000,650,1)).toEqual({width:1000,height:650,limited:false});expect(rasterSize(1000,650,2)).toEqual({width:2000,height:1300,limited:false})});
 it('fits large tree dimensions inside the explicit 4096 pixel bound',()=>{expect(rasterSize(1000,10000,2)).toEqual({width:410,height:4096,limited:true});expect(rasterSize(100000,1,1)).toEqual({width:4096,height:1,limited:true})});
 it('rejects invalid dimensions and unsupported resolutions',()=>{for(const [w,h,s]of [[0,1,1],[1,-1,1],[Infinity,2,1],[2,NaN,1],[1,1,3]])expect(()=>rasterSize(w,h,s)).toThrow()});
});

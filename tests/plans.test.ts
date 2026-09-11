import {describe,it,expect} from 'vitest';
import {buildPlans,displayPlans} from '../src/lib/ffmpeg';
import defaults from '../src/lib/defaults';
import util from '../src/lib/util';
import {deepMerge} from '../src/lib/merge';
import {withSubtitles,isSubtitle,isMedia} from '../src/attachments';
const data=import.meta.glob('../src/lib/presets/*.json',{eager:true,import:'default'});
const build=(form:any)=>buildPlans(util.transform(form) as never);
describe('Native command planning',()=>{
 for(const [name,preset] of Object.entries(data))it(`builds ${name}`,()=>{const plans=build(deepMerge(defaults(),preset as any));expect(plans.length).toBeGreaterThan(0);expect(plans.flat().every(a=>typeof a==='string'&&!/undefined|NaN/.test(a))).toBe(true);expect(plans.flat()).not.toContain('&&')});
 it('preserves literal Windows paths and metacharacters',()=>{const f=defaults();f.io={input:"C:\\vidéos\\a & b's $clip.mp4",output:'C:\\out file.mp4'};const p=build(f);expect(p[0][1]).toBe(f.io.input);expect(p[0].at(-1)).toBe(f.io.output);expect(displayPlans(p)).toContain("b''s $clip")});
 it('creates independent two-pass plans',()=>{const f=defaults();f.video.pass='2';f.video.bitrate='1000k' as never;const p=build(f);expect(p).toHaveLength(2);expect(p[0].slice(-3)).toEqual(['-f','null','-']);expect(p[1].at(-1)).toBe('output.mp4');expect(p[0]).toContain('-an');expect(p[0]).toContain('-pass')});
 it('retains lossless CRF zero',()=>{const f=defaults();f.video.pass='crf';f.video.crf=0;const p=build(f)[0];expect(p[p.indexOf('-crf')+1]).toBe('0')});
 it('rejects copy plus filters',()=>{const f=defaults();f.video.codec='copy';f.video.size='1280';expect(()=>build(f)).toThrow()});
 it('rejects two-pass without bitrate',()=>{const f=defaults();f.video.pass='2';expect(()=>build(f)).toThrow()});
 it('maps subtitle inputs only on final pass',()=>{const f=defaults();f.video.pass='2';f.video.bitrate='1000k' as never;const p=withSubtitles(build(f),['C:\\subs & é.srt'],'mp4');expect(p[0]).not.toContain('-c:s');expect(p[1]).toContain('mov_text');expect(p[1]).toContain('1:s:0');expect(p[1][3]).toBe('C:\\subs & é.srt')});
 it('rejects unsupported subtitle containers',()=>expect(()=>withSubtitles(build(defaults()),['a.srt'],'webm')).toThrow());
 it('recognizes media and subtitle extensions',()=>{expect(isSubtitle('a.SRT')).toBe(true);expect(isMedia('a.MKV')).toBe(true);expect(isMedia('file.exe')).toBe(false)});
});

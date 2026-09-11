import { useCallback, useState } from 'react'
import createDefaultForm from '@/lib/defaults'
import { reconcile } from '@/lib/reconcile'
import type { IFFMpegOptionsForm } from '@/lib/types'
export function useFfmpegForm(){
 const [form,setForm]=useState(()=>createDefaultForm() as unknown as IFFMpegOptionsForm);
 const update=useCallback(<S extends keyof IFFMpegOptionsForm>(section:S,patch:Partial<IFFMpegOptionsForm[S]>)=>setForm(prev=>({...prev,[section]:{...prev[section],...patch}})),[]);
 const updateFormat=(patch:Partial<IFFMpegOptionsForm['format']>)=>setForm(p=>reconcile({...p,format:{...p.format,...patch}}));
 const updateVideo=(patch:Partial<IFFMpegOptionsForm['video']>)=>setForm(p=>patch.codec?reconcile({...p,video:{...p.video,...patch}}):({...p,video:{...p.video,...patch}}));
 return {form,setForm,update,updateFormat,updateVideo,reset:()=>setForm(createDefaultForm() as unknown as IFFMpegOptionsForm)};
}

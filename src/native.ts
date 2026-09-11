import {invoke,isTauri} from '@tauri-apps/api/core';
export const desktop=isTauri();
export interface Saved {id:string;name:string;data:Record<string,unknown>}
export interface Installed {id:string;name:string;path:string;managed:boolean;checksum:string}
export interface Job {id:string;output:string;binary:string;plans:string[][];status:string;progress:number;duration:number;pass:number;logs:string[];created:number}
export interface Snapshot {config:{active:string;versions:Installed[];presets:Saved[]};jobs:Job[];installing:string}
export interface Release {tag:string;name:string;url:string;size:number;digest:string}
export const empty:Snapshot={config:{active:'',versions:[],presets:[]},jobs:[],installing:''};
export const native=<T,>(cmd:string,args?:Record<string,unknown>)=>desktop?invoke<T>(cmd,args):Promise.reject(new Error('Cette action nécessite l’application de bureau. Lancez npm run desktop.'));

import {t} from '@/i18n';
import Field from '../ui/Field';
import Select from '../ui/Select';
import form from '@/lib/form';
import type {IFFMpegOptionsForm} from '@/lib/types';
export default function OptionsSection({value,onChange}:{value:IFFMpegOptionsForm['options'];onChange:(p:Partial<IFFMpegOptionsForm['options']>)=>void}){
 const extra=value.extra as unknown as string[];
 return <div className="space-y-5"><p className="text-sm text-muted">{t("Les fichiers existants sont protégés par défaut.")}</p>{[{value:'y',text:'Autoriser l’écrasement de la sortie'},{value:'f',text:'Forcer le format du conteneur'},{value:'hide_banner',text:'Masquer la bannière FFmpeg'},{value:'report',text:'Écrire un rapport FFmpeg'}].map(o=><label key={o.value} className="flex gap-3 items-center"><input className="checkbox checkbox-primary checkbox-sm" type="checkbox" checked={extra.includes(o.value)} onChange={e=>onChange({extra:(e.target.checked?[...extra.filter(f=>f!=='n'),o.value]:extra.filter(f=>f!==o.value)) as unknown as string})}/>{o.text}</label>)}<Field label="Log level" htmlFor="loglevel"><Select id="loglevel" value={value.loglevel} options={form.logLevels} onChange={loglevel=>onChange({loglevel})}/></Field></div>
}

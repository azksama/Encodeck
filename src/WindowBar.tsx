import {Minus,Square,X,Terminal,Menu} from 'lucide-react';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {desktop} from './native';
import {t} from './i18n';
export default function WindowBar(){return <div className="window-bar"><div className="window-brand" data-tauri-drag-region onDoubleClick={()=>desktop&&getCurrentWindow().toggleMaximize()}><span data-tauri-drag-region>Encodeck</span></div><button className="window-menu" onClick={()=>window.dispatchEvent(new Event("encodeck-palette"))}><Menu size={15}/>{t("Actions")}<kbd>Ctrl ⇧ P</kbd></button><div className="window-controls"><button aria-label={t('Minimize')} onClick={()=>desktop&&getCurrentWindow().minimize()}><Minus size={14}/></button><button aria-label={t('Maximize')} onClick={()=>desktop&&getCurrentWindow().toggleMaximize()}><Square size={12}/></button><button aria-label={t('Close')} onClick={()=>desktop&&getCurrentWindow().close()}><X size={16}/></button></div></div>}



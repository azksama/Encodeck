import {createRoot} from 'react-dom/client';
import App from './App';
import WindowBar from './WindowBar';
import './style.css';
createRoot(document.getElementById('root')!).render(<><WindowBar/><App/></>);

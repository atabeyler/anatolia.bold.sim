import React from 'react';
import ReactDOM from 'react-dom/client';
import axios from 'axios';
import App from './App';
import NativeModeGate from './components/layout/NativeModeGate';
import BrowserModeGate from './components/layout/BrowserModeGate';
import { installWasmLocalAdapter } from './wasmLocal/apiAdapter';
import './index.css';

axios.defaults.withCredentials = true;
// A no-op until activateWasmLocalMode() flips the flag it checks per-request
// (see wasmLocal/mode.ts) -- every existing call, relative or absolute,
// behaves exactly as before until then.
installWasmLocalAdapter();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <NativeModeGate>
      <BrowserModeGate>
        <App />
      </BrowserModeGate>
    </NativeModeGate>
  </React.StrictMode>
);

import { attachConsole } from '@tauri-apps/plugin-log'
import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Reenvía los logs de Rust a la consola del webview en desarrollo.
if (import.meta.env.DEV) attachConsole()

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app

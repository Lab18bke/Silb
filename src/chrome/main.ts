import '@fontsource-variable/source-serif-4/opsz.css';
import '../styles/base.css';
import { mount } from 'svelte';
import App from './App.svelte';

document.addEventListener('contextmenu', (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest('input, textarea, [contenteditable="true"]')) e.preventDefault();
});

document.addEventListener('dragover', (e) => e.preventDefault());
document.addEventListener('drop', (e) => e.preventDefault());

export default mount(App, { target: document.getElementById('app')! });

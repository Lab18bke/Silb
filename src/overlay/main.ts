import '@fontsource-variable/source-serif-4/opsz.css';
import '../styles/base.css';
import { mount } from 'svelte';
import Overlay from './Overlay.svelte';

document.addEventListener('contextmenu', (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest('input, textarea')) e.preventDefault();
});

export default mount(Overlay, { target: document.getElementById('app')! });

<script lang="ts">
  import Icon from './Icon.svelte';
  import { api } from '../lib/api';
  import { browser } from '../chrome/state.svelte';
</script>

<div class="controls">
  <button type="button" aria-label="Minimize" title="Minimize" onclick={() => api.windowControl('minimize')}>
    <Icon name="win-min" size={12} stroke={1} />
  </button>
  <button
    type="button"
    aria-label={browser.maximized ? 'Restore' : 'Maximize'}
    title={browser.maximized ? 'Restore' : 'Maximize'}
    onclick={() => api.windowControl('toggle-maximize')}
  >
    <Icon name={browser.maximized ? 'win-restore' : 'win-max'} size={12} stroke={1} />
  </button>
  <button type="button" class="close" aria-label="Close" title="Close" onclick={() => api.windowControl('close')}>
    <Icon name="win-close" size={12} stroke={1} />
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-self: stretch;
  }
  button {
    width: 46px;
    border: 0;
    background: transparent;
    color: var(--ink-2);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s;
  }
  button:hover {
    background: var(--hover);
  }
  .close:hover {
    background: #c42b1c;
    color: #fff;
  }
</style>

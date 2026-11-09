<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import { api, errorText } from '../lib/api';
  import { browser } from '../chrome/state.svelte';
  import { bus } from '../chrome/actions';
  import { toast } from '../chrome/popover';

  let input: HTMLInputElement;
  let text = $state('');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!text.trim() || browser.active == null) return;
    try {
      await api.tabNavigate(browser.active, text);
    } catch (err) {
      toast(errorText(err));
    }
  }

  onMount(() => {
    const focus = () => input?.focus();
    bus.addEventListener('focus-page', focus);
    requestAnimationFrame(focus);
    return () => bus.removeEventListener('focus-page', focus);
  });
</script>

<div class="private">
  <div class="column">
    <div class="mark"><Icon name="mask" size={30} stroke={1.2} /></div>
    <h1>Private window</h1>
    <form onsubmit={submit} class="search">
      <Icon name="search" size={18} />
      <input bind:this={input} bind:value={text} placeholder="Search or type an address" aria-label="Search or type an address" spellcheck="false" autocomplete="off" />
    </form>

    <div class="facts">
      <section>
        <h2>When you close all private windows, Silb deletes</h2>
        <ul>
          <li>Pages you visited in these windows</li>
          <li>Cookies and site data created in these windows</li>
          <li>Permissions and zoom levels you set for sites here</li>
          <li>The list of files downloaded here</li>
        </ul>
      </section>
      <section>
        <h2>Silb keeps</h2>
        <ul>
          <li>Files you download — they stay in your downloads folder</li>
          <li>Bookmarks you create — they are added to your regular bookmarks</li>
        </ul>
      </section>
    </div>

    <p class="caveat">
      Private browsing keeps activity off this device. It doesn’t make you anonymous: websites you visit, your employer or
      school, and your internet provider can still see what you do.
    </p>
  </div>
</div>

<style>
  .private {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    background: var(--surface);
  }
  .column {
    width: min(620px, calc(100% - 48px));
    margin: max(8vh, 40px) auto 48px;
  }
  .mark {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  h1 {
    font-family: var(--font-display);
    font-weight: 400;
    font-size: 42px;
    letter-spacing: -0.02em;
    margin: 22px 0 26px;
  }
  .search {
    height: 50px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 18px;
    border-radius: var(--radius-l);
    background: var(--raised);
    border: 1px solid var(--line-strong);
    color: var(--muted);
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
  .search input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15px;
  }
  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
    margin-top: 36px;
  }
  h2 {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    margin: 0 0 10px;
  }
  ul {
    margin: 0;
    padding-left: 18px;
    color: var(--ink-2);
    line-height: 1.75;
  }
  .caveat {
    margin-top: 32px;
    padding-top: 20px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    line-height: 1.65;
  }
  @media (max-width: 640px) {
    .facts {
      grid-template-columns: 1fr;
    }
  }
</style>

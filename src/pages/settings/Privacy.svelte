<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import ClearDataDialog from './ClearDataDialog.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { api, errorText } from '../../lib/api';
  import { on } from '../../lib/events';
  import { PERMISSION_LABELS } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { browser } from '../../chrome/state.svelte';
  import { toast } from '../../chrome/popover';
  import type { SiteData, SiteSetting } from '../../lib/types';

  let { openClear = false }: { openClear?: boolean } = $props();

  const s = $derived(settings.value!);
  let showClear = $state(untrack(() => openClear));
  let sites = $state<SiteData[] | null>(null);
  let siteFilter = $state('');
  let perms = $state<SiteSetting[]>([]);

  const levels = {
    off: 'Websites can track you across sites. Only use this if a site doesn’t work otherwise.',
    basic: 'Blocks trackers known to be harmful, such as cryptomining and fingerprinting. Most ads stay personalised.',
    balanced: 'Also blocks trackers from sites you haven’t visited. Content and ads are likely to be less personalised. Recommended.',
    strict: 'Blocks most trackers across sites. Some parts of sites may not work.',
  } as const;

  const filteredSites = $derived(sites?.filter((x) => x.domain.includes(siteFilter.trim().toLowerCase())) ?? []);

  const byOrigin = $derived.by(() => {
    const map = new Map<string, SiteSetting[]>();
    for (const p of perms) map.set(p.origin, [...(map.get(p.origin) ?? []), p]);
    return [...map.entries()];
  });

  async function loadSites() {
    try {
      sites = await api.siteDataList();
    } catch (e) {
      sites = [];
      toast(errorText(e));
    }
  }

  async function loadPerms() {
    perms = await api.siteSettingsList();
  }

  async function clearSite(domain: string) {
    try {
      await api.siteDataClear(domain);
      sites = sites?.filter((x) => x.domain !== domain) ?? null;
      toast(`Cleared data for ${domain}`);
    } catch (e) {
      toast(errorText(e));
    }
  }

  async function setPerm(origin: string, kind: string, value: string) {
    try {
      await api.siteSettingSet(origin, kind, value === 'ask' ? null : (value as 'allow' | 'block'));
      await loadPerms();
    } catch (e) {
      toast(errorText(e));
    }
  }

  function closeClear() {
    showClear = false;
    if (openClear && browser.active != null) api.tabNavigate(browser.active, 'silb://settings/privacy');
  }

  onMount(() => {
    loadPerms();
    return on('site-settings-changed', loadPerms);
  });
</script>

<Group title="Browsing data">
  <Row label="Clear browsing data" description="History, cookies, cached files and download history, for a time range you choose.">
    <button class="btn" onclick={() => (showClear = true)}>Clear data…</button>
  </Row>
</Group>

<Group title="Tracking prevention">
  <Row label="Block trackers" description={levels[s.trackingPrevention]} stacked>
    <Segmented
      label="Tracking prevention level"
      value={s.trackingPrevention}
      options={[
        { value: 'off', label: 'Off' },
        { value: 'basic', label: 'Basic' },
        { value: 'balanced', label: 'Balanced' },
        { value: 'strict', label: 'Strict' },
      ]}
      onchange={(v) => settings.update({ trackingPrevention: v })}
    />
  </Row>
</Group>

<Group title="Pop-ups">
  <Row label="Block pop-ups" description="Pop-ups that open without a click are blocked. You can allow them for a site from the address bar.">
    <Toggle label="Block pop-ups" checked={s.blockPopups} onchange={(v) => settings.update({ blockPopups: v })} />
  </Row>
</Group>

<Group title="Cookies and site data">
  <Row
    label="Sites that store cookies"
    description="Cookies keep you signed in and remember preferences. They’re kept between sessions and expire when the site says so. Private windows use separate storage."
    stacked
  >
    {#if sites === null}
      <button class="btn" onclick={loadSites}>Show sites</button>
    {:else}
      <input class="field filter" bind:value={siteFilter} placeholder="Filter sites" aria-label="Filter sites" />
      <ul class="sites">
        {#each filteredSites as site (site.domain)}
          <li>
            <span class="domain">{site.domain}</span>
            <span class="count">{site.cookies} {site.cookies === 1 ? 'cookie' : 'cookies'}</span>
            <button class="btn quiet" onclick={() => clearSite(site.domain)}>Clear</button>
          </li>
        {:else}
          <li class="none">{sites.length ? 'No sites match.' : 'No sites have stored cookies.'}</li>
        {/each}
      </ul>
    {/if}
  </Row>
</Group>

<Group title="Site permissions">
  <Row
    label="Permissions you’ve granted or blocked"
    description="Sites ask before using your camera, microphone, location, notifications or clipboard. Choices you remember are listed here."
    stacked
  >
    {#if byOrigin.length}
      <div class="perms">
        {#each byOrigin as [origin, list] (origin)}
          <div class="origin">
            <div class="oname">{origin}</div>
            {#each list as p (p.kind)}
              <div class="perm">
                <span>{PERMISSION_LABELS[p.kind] ?? p.kind}</span>
                <select class="field" value={p.value} onchange={(e) => setPerm(origin, p.kind, (e.currentTarget as HTMLSelectElement).value)} aria-label="{PERMISSION_LABELS[p.kind]} for {origin}">
                  <option value="allow">Allow</option>
                  <option value="block">Block</option>
                  <option value="ask">{p.kind === 'popups' || p.kind === 'javascript' ? 'Default' : 'Ask'}</option>
                </select>
                <IconButton icon="close-small" label="Reset" onclick={() => setPerm(origin, p.kind, 'ask')} />
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {:else}
      <p class="none">No site permissions saved yet.</p>
    {/if}
  </Row>
</Group>

<Group title="Private browsing and connections">
  <Row
    label="Private windows"
    description="Private windows use a separate, temporary profile. History, cookies and site data from private windows are deleted when the last private window closes. Private browsing doesn’t make you anonymous online."
  />
  <Row
    label="Secure connections"
    description="A lock in the address bar means the page was loaded over HTTPS. “Not secure” means the connection is unencrypted (HTTP) or the site’s certificate had a problem. A lock doesn’t mean the site itself is trustworthy."
  />
</Group>

{#if showClear}
  <ClearDataDialog
    onclose={closeClear}
    ondone={(m) => {
      toast(m);
      closeClear();
      if (sites) loadSites();
    }}
  />
{/if}

<style>
  .filter {
    width: 240px;
  }
  .sites {
    list-style: none;
    margin: 4px 0 6px;
    padding: 0;
    width: 100%;
    max-height: 320px;
    overflow-y: auto;
  }
  .sites li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0 4px 2px;
    border-bottom: 1px solid var(--line);
  }
  .domain {
    flex: 1;
  }
  .count {
    color: var(--muted);
    font-size: 12px;
  }
  .none {
    color: var(--muted);
    margin: 0;
    padding: 6px 0;
  }
  .perms {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 4px;
  }
  .oname {
    font-weight: 600;
    margin-bottom: 4px;
  }
  .perm {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0 4px 12px;
  }
  .perm select {
    width: 128px;
  }
  .perm span {
    flex: 1;
    color: var(--ink-2);
  }
</style>

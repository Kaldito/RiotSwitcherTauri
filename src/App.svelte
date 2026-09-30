<script lang="ts">
  import { onMount } from 'svelte'
  import { t } from './lib/i18n/index.svelte'
  import { app, init } from './lib/state/app.svelte'
  import { goHome, openSettings, view } from './lib/state/view.svelte'
  import BootView from './lib/views/BootView.svelte'
  import HomeView from './lib/views/HomeView.svelte'
  import ProfileFormView from './lib/views/ProfileFormView.svelte'
  import SettingsView from './lib/views/SettingsView.svelte'

  onMount(() => {
    init()
  })

  const needsBoot = $derived(!app.boot?.riotClientValid)
  const runningLabel = $derived(
    app.runtime.runningProfile ? t('app.running', app.runtime.runningProfile) : '',
  )
</script>

{#if app.loadError}
  <main><p class="error">{app.loadError}</p></main>
{:else if !app.ready}
  <main><p>…</p></main>
{:else if needsBoot}
  <BootView />
{:else}
  <nav class="row">
    <strong>RiotSwitcher</strong>
    <button type="button" aria-current={view.name === 'home'} onclick={goHome}>{t('app.profiles')}</button>
    <button type="button" aria-current={view.name === 'settings'} onclick={openSettings}>{t('app.settings')}</button>
    <span class="status">{runningLabel}</span>
  </nav>
  <main>
    {#if view.name === 'form'}
      {#key view.editing}
        <ProfileFormView />
      {/key}
    {:else if view.name === 'settings'}
      <SettingsView />
    {:else}
      <HomeView />
    {/if}
  </main>
{/if}

<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { LANGUAGES, t } from '../i18n/index.svelte'
  import { setRiotClientLocation } from '../ipc/commands'
  import { errorMessage } from '../ipc/errors'
  import { app, patchConfig, refreshBoot } from '../state/app.svelte'

  let error = $state<string | null>(null)
  const locked = $derived(app.runtime.runningProfile !== null || app.runtime.phase === 'busy')

  async function changeLanguage(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value
    try {
      await patchConfig({ selectedLanguage: value })
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function changeRiotFolder() {
    const picked = await open({ directory: true, title: t('boot.title') })
    if (typeof picked !== 'string') return
    error = null
    try {
      app.config = await setRiotClientLocation(picked)
      await refreshBoot()
    } catch (e) {
      error = errorMessage(e)
    }
  }
</script>

<section class="narrow">
  <h2>{t('app.settings')}</h2>

  <label>
    {t('settings.language')}
    <select value={app.config?.selectedLanguage ?? ''} onchange={changeLanguage}>
      <option value="">{t('settings.system_language')}</option>
      {#each LANGUAGES as lang (lang.code)}
        <option value={lang.code}>{lang.label}</option>
      {/each}
    </select>
  </label>

  <h3>{t('settings.riot_folder')}</h3>
  <p><code>{app.config?.riotClientLocation}</code></p>
  <button type="button" onclick={changeRiotFolder} disabled={locked}>{t('settings.change')}</button>
  {#if locked}
    <p class="hint">{t('settings.locked')}</p>
  {/if}

  <h3>{t('settings.data_folder')}</h3>
  <p><code>{app.boot?.dataDir}</code></p>

  {#if error}<p class="error">{error}</p>{/if}
</section>

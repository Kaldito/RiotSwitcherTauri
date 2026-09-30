<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { t } from '../i18n/index.svelte'
  import { setRiotClientLocation } from '../ipc/commands'
  import { errorMessage } from '../ipc/errors'
  import { app, refreshBoot } from '../state/app.svelte'

  let saving = $state(false)
  let error = $state<string | null>(null)

  async function useFolder(path: string) {
    saving = true
    error = null
    try {
      app.config = await setRiotClientLocation(path)
      await refreshBoot()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      saving = false
    }
  }

  async function browse() {
    const picked = await open({ directory: true, title: t('boot.title') })
    if (typeof picked === 'string') await useFolder(picked)
  }
</script>

<main class="boot">
  <h1>RiotSwitcher</h1>
  <h2>{t('boot.title')}</h2>
  <p>{t('boot.hint')}</p>

  {#if app.boot?.suggestedRiotClientLocation}
    <p>
      {t('boot.detected', app.boot.suggestedRiotClientLocation)}
      <button
        type="button"
        disabled={saving}
        onclick={() => useFolder(app.boot!.suggestedRiotClientLocation!)}>{t('boot.use_detected')}</button
      >
    </p>
  {/if}

  <button type="button" onclick={browse} disabled={saving}>{t('boot.browse')}</button>
  {#if error}<p class="error">{error}</p>{/if}
</main>

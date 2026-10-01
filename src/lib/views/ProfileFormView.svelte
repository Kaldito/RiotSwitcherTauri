<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { backgroundSrc, customBackgroundPath } from '../backgrounds'
  import { t } from '../i18n/index.svelte'
  import { createProfile, importBackgroundImage, importLeagueIcon, updateProfile } from '../ipc/commands'
  import { errorMessage } from '../ipc/errors'
  import type { BackgroundRef } from '../ipc/types'
  import { app } from '../state/app.svelte'
  import { goHome, view } from '../state/view.svelte'

  const original = app.profiles.find((p) => p.name === view.editing) ?? null

  let name = $state(original?.name ?? '')
  let description = $state(original?.description ?? '')
  let background = $state<BackgroundRef>(original?.background ?? { kind: 'none' })
  let saving = $state(false)
  let fetchingIcon = $state(false)
  let error = $state<string | null>(null)

  const previewSrc = $derived(
    background.kind === 'custom' && app.boot
      ? backgroundSrc(background, customBackgroundPath(app.boot.dataDir, background.file))
      : null,
  )

  async function chooseImage() {
    const picked = await open({
      title: t('form.choose_image'),
      filters: [{ name: t('form.images'), extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    })
    if (typeof picked !== 'string') return
    try {
      background = await importBackgroundImage(picked, name.trim() || null)
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function useLeagueIcon() {
    fetchingIcon = true
    error = null
    try {
      background = await importLeagueIcon(name.trim() || null)
    } catch (e) {
      error = errorMessage(e)
    } finally {
      fetchingIcon = false
    }
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    saving = true
    error = null
    const typed = name.trim() || null
    try {
      if (original) {
        await updateProfile(original.name, { name: typed, description, background })
      } else {
        await createProfile({ name: typed, description, background })
      }
      goHome()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      saving = false
    }
  }
</script>

<section class="narrow">
  <h2>{original ? t('form.edit_title') : t('form.new_title')}</h2>
  <form onsubmit={submit}>
    <label>
      {t('form.name')}
      <input
        bind:value={name}
        maxlength="64"
        placeholder={original ? original.name : t('form.name_placeholder')}
      />
    </label>

    <label>
      {t('form.description')}
      <textarea bind:value={description} maxlength="200" rows="2"></textarea>
    </label>

    <fieldset>
      <legend>{t('form.background')}</legend>
      {#if previewSrc}
        <img class="preview" src={previewSrc} alt="" />
      {/if}
      <div class="row">
        <button type="button" onclick={chooseImage}>{t('form.choose_image')}</button>
        <button type="button" onclick={useLeagueIcon} disabled={fetchingIcon}>
          {t('form.league_icon')}
        </button>
        {#if background.kind === 'custom'}
          <button type="button" onclick={() => (background = { kind: 'none' })}>
            {t('form.no_background')}
          </button>
        {/if}
      </div>
      <p class="hint">{t('form.league_icon_hint')}</p>
    </fieldset>

    {#if error}<p class="error">{error}</p>{/if}
    <div class="row">
      <button type="submit" class="primary" disabled={saving}>{original ? t('form.save') : t('form.create')}</button>
      <button type="button" onclick={goHome} disabled={saving}>{t('form.cancel')}</button>
    </div>
  </form>
</section>

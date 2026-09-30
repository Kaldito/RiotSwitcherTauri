<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog'
  import { backgroundSrc } from '../backgrounds'
  import { t } from '../i18n/index.svelte'
  import { deleteProfile, reorderProfiles } from '../ipc/commands'
  import { errorMessage } from '../ipc/errors'
  import type { Profile } from '../ipc/types'
  import { app, play, stop } from '../state/app.svelte'
  import { openForm } from '../state/view.svelte'

  interface Props {
    profile: Profile
    index: number
    onError: (message: string) => void
  }

  let { profile, index, onError }: Props = $props()

  const running = $derived(app.runtime.runningProfile === profile.name)
  const busy = $derived(app.runtime.phase === 'busy' || (app.swap?.active ?? false))
  const thumb = $derived(backgroundSrc(profile.background, profile.backgroundPath))

  async function remove() {
    const confirmed = await ask(t('profile.delete_confirm', profile.name), {
      title: t('profile.delete_title'),
      kind: 'warning',
    })
    if (!confirmed) return
    try {
      app.profiles = await deleteProfile(profile.name)
    } catch (e) {
      onError(errorMessage(e))
    }
  }

  async function move(offset: -1 | 1) {
    const names = app.profiles.map((p) => p.name)
    const target = index + offset
    if (target < 0 || target >= names.length) return
    ;[names[index], names[target]] = [names[target], names[index]]
    try {
      app.profiles = await reorderProfiles(names)
    } catch (e) {
      onError(errorMessage(e))
    }
  }
</script>

<article class="card" class:running>
  {#if thumb}
    <img class="thumb" src={thumb} alt="" />
  {:else}
    <div class="thumb placeholder" aria-hidden="true">{profile.name.charAt(0).toUpperCase()}</div>
  {/if}
  <div class="info">
    <strong>{profile.name}</strong>
    {#if running}<span class="badge">{t('profile.running')}</span>{/if}
    {#if profile.description}<p>{profile.description}</p>{/if}
  </div>
  <div class="actions">
    {#if running}
      <button type="button" onclick={stop} disabled={busy}>{t('profile.stop')}</button>
    {:else}
      <button type="button" onclick={() => play(profile.name)} disabled={busy}>{t('profile.play')}</button>
    {/if}
    <button type="button" onclick={() => openForm(profile.name)} disabled={busy}>{t('profile.edit')}</button>
    <button type="button" onclick={remove} disabled={busy || running}>{t('profile.delete')}</button>
    <button
      type="button"
      onclick={() => move(-1)}
      disabled={busy || index === 0}
      title={t('profile.move_up')}
      aria-label={t('profile.move_up')}>↑</button
    >
    <button
      type="button"
      onclick={() => move(1)}
      disabled={busy || index === app.profiles.length - 1}
      title={t('profile.move_down')}
      aria-label={t('profile.move_down')}>↓</button
    >
  </div>
</article>

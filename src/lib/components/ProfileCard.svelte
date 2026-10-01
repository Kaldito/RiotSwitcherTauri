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

<article class="tile" class:running data-index={index}>
  <button
    type="button"
    class="avatar"
    onclick={() => (running ? stop() : play(profile.name))}
    disabled={busy}
    aria-label={`${running ? t('profile.stop') : t('profile.play')}: ${profile.name}`}
  >
    {#if thumb}
      <img src={thumb} alt="" />
    {:else}
      <span class="letter" aria-hidden="true">{profile.name.charAt(0).toUpperCase()}</span>
    {/if}
    <span class="overlay">{running ? t('profile.stop') : t('profile.play')}</span>
  </button>
  <strong class="name" title={profile.name}>{profile.name}</strong>
  {#if running}<span class="badge">{t('profile.running')}</span>{/if}
  {#if profile.description}<p class="desc">{profile.description}</p>{/if}
  <div class="tools">
    <button
      type="button"
      onclick={() => move(-1)}
      disabled={busy || index === 0}
      title={t('profile.move_up')}
      aria-label={t('profile.move_up')}>←</button
    >
    <button
      type="button"
      onclick={() => move(1)}
      disabled={busy || index === app.profiles.length - 1}
      title={t('profile.move_down')}
      aria-label={t('profile.move_down')}>→</button
    >
    <button
      type="button"
      onclick={() => openForm(profile.name)}
      disabled={busy}
      title={t('profile.edit')}
      aria-label={t('profile.edit')}>✎</button
    >
    <button
      type="button"
      class="danger"
      onclick={remove}
      disabled={busy || running}
      title={t('profile.delete')}
      aria-label={t('profile.delete')}>✕</button
    >
  </div>
</article>

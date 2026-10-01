<script lang="ts">
  import ProfileCard from '../components/ProfileCard.svelte'
  import { t } from '../i18n/index.svelte'
  import { reorderProfiles } from '../ipc/commands'
  import { errorMessage } from '../ipc/errors'
  import { app, patchConfig } from '../state/app.svelte'
  import { openForm } from '../state/view.svelte'

  let error = $state<string | null>(null)
  const busy = $derived(app.runtime.phase === 'busy' || (app.swap?.active ?? false))

  // Arrastrar para reordenar. Se hace con pointer events porque el drag and drop
  // nativo de HTML choca con el manejador de archivos soltados de Tauri.
  const DRAG_THRESHOLD = 8
  interface Drag {
    pointerId: number
    el: HTMLElement
    from: number
    to: number
    x: number
    y: number
    active: boolean
  }
  let tiles: HTMLDivElement
  let drag: Drag | null = null
  let suppressClick = false

  function profileTiles(): HTMLElement[] {
    return Array.from(tiles.querySelectorAll<HTMLElement>('[data-index]'))
  }

  function clearDropMarks() {
    for (const el of profileTiles()) delete el.dataset.drop
  }

  function endDrag() {
    if (!drag) return
    if (drag.active && tiles.hasPointerCapture(drag.pointerId)) tiles.releasePointerCapture(drag.pointerId)
    drag.el.style.transform = ''
    delete drag.el.dataset.dragging
    clearDropMarks()
    drag = null
  }

  function onPointerDown(event: PointerEvent) {
    if (busy || event.button !== 0) return
    const target = event.target as HTMLElement
    if (target.closest('.tools')) return
    const el = target.closest<HTMLElement>('[data-index]')
    if (!el) return
    const from = Number(el.dataset.index)
    drag = { pointerId: event.pointerId, el, from, to: from, x: event.clientX, y: event.clientY, active: false }
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return
    const dx = event.clientX - drag.x
    const dy = event.clientY - drag.y
    if (!drag.active) {
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return
      drag.active = true
      tiles.setPointerCapture(drag.pointerId)
      drag.el.dataset.dragging = ''
    }
    drag.el.style.transform = `translate(${dx}px, ${dy}px)`

    clearDropMarks()
    drag.to = drag.from
    for (const el of profileTiles()) {
      if (el === drag.el) continue
      const r = el.getBoundingClientRect()
      if (event.clientX < r.left || event.clientX > r.right || event.clientY < r.top || event.clientY > r.bottom) continue
      drag.to = Number(el.dataset.index)
      el.dataset.drop = drag.to > drag.from ? 'after' : 'before'
      break
    }
  }

  async function onPointerUp(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return
    const { active, from, to } = drag
    endDrag()
    if (!active) return
    // El click que sigue a soltar no debe iniciar el perfil.
    suppressClick = true
    setTimeout(() => (suppressClick = false), 0)
    if (to === from) return
    const names = app.profiles.map((p) => p.name)
    names.splice(to, 0, ...names.splice(from, 1))
    try {
      app.profiles = await reorderProfiles(names)
    } catch (e) {
      error = errorMessage(e)
    }
  }

  function onClickCapture(event: MouseEvent) {
    if (!suppressClick) return
    event.stopPropagation()
    event.preventDefault()
  }

  async function hideWarning() {
    try {
      await patchConfig({ showAddProfileWarning: false })
    } catch (e) {
      error = errorMessage(e)
    }
  }
</script>

<section class="picker">
  <h2>{t('home.title')}</h2>
  <p class="subtitle">{app.profiles.length === 0 ? t('home.empty') : t('home.subtitle')}</p>

  {#if app.config?.showAddProfileWarning}
    <p class="notice">
      {t('home.stay_signed_in')}
      <button type="button" onclick={hideWarning}>{t('home.dismiss')}</button>
    </p>
  {/if}

  {#if app.swap?.error}<p class="error">{app.swap.error}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}

  <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
  <div
    class="tiles"
    bind:this={tiles}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={endDrag}
    onclickcapture={onClickCapture}
  >
    {#each app.profiles as profile, index (profile.name)}
      <ProfileCard {profile} {index} onError={(m) => (error = m)} />
    {/each}
    <div class="tile">
      <button
        type="button"
        class="avatar add"
        onclick={() => openForm(null)}
        disabled={busy}
        title={t('home.new_profile')}
        aria-label={t('home.new_profile')}>+</button
      >
    </div>
  </div>
</section>

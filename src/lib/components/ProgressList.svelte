<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { errorMessage } from '../ipc/errors'
  import type { SwapProgress } from '../ipc/types'
  import { app, dismissSwap } from '../state/app.svelte'

  type StepEvent = Extract<SwapProgress, { kind: 'step' }>
  type WarningEvent = Extract<SwapProgress, { kind: 'warning' }>

  const swap = $derived(app.swap)
  const steps = $derived(
    (swap?.events ?? []).filter((e): e is StepEvent => e.kind === 'step'),
  )
  const warnings = $derived(
    (swap?.events ?? []).filter((e): e is WarningEvent => e.kind === 'warning'),
  )
  const done = $derived(swap?.events.some((e) => e.kind === 'done') ?? false)
</script>

{#if swap}
  <section class="progress" aria-live="polite">
    <h3>
      {swap.action === 'play' ? t('profile.play') : t('profile.stop')}{swap.profile ? `: ${swap.profile}` : ''}
    </h3>
    <ol>
      {#each steps as s, i (s.index)}
        <li class:current={swap.active && i === steps.length - 1}>
          {t(`progress.step.${s.step}`)}
          <small>({s.index}/{s.total})</small>
        </li>
      {/each}
    </ol>
    {#each warnings as w, i (i)}
      <p class="warning">
        {errorMessage({ code: w.code, message: w.code, details: w.path ? { path: w.path } : undefined })}
      </p>
    {/each}
    {#if swap.error}
      <p class="error">{swap.error}</p>
    {:else if done}
      <p class="ok">{t('progress.done')}</p>
    {/if}
    {#if !swap.active}
      <button type="button" onclick={dismissSwap}>{t('common.close')}</button>
    {/if}
  </section>
{/if}

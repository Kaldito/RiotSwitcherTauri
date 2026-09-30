<script lang="ts">
  import ProfileCard from '../components/ProfileCard.svelte'
  import ProgressList from '../components/ProgressList.svelte'
  import { t } from '../i18n/index.svelte'
  import { errorMessage } from '../ipc/errors'
  import { app, patchConfig } from '../state/app.svelte'
  import { openForm } from '../state/view.svelte'

  let error = $state<string | null>(null)

  async function hideWarning() {
    try {
      await patchConfig({ showAddProfileWarning: false })
    } catch (e) {
      error = errorMessage(e)
    }
  }
</script>

<section>
  <header class="row">
    <h2>{t('app.profiles')}</h2>
    <button type="button" onclick={() => openForm(null)}>{t('home.new_profile')}</button>
  </header>

  {#if app.config?.showAddProfileWarning}
    <p class="notice">
      {t('home.stay_signed_in')}
      <button type="button" onclick={hideWarning}>{t('home.dismiss')}</button>
    </p>
  {/if}

  <ProgressList />
  {#if error}<p class="error">{error}</p>{/if}

  {#if app.profiles.length === 0}
    <p>{t('home.empty')}</p>
  {:else}
    <div class="cards">
      {#each app.profiles as profile, index (profile.name)}
        <ProfileCard {profile} {index} onError={(m) => (error = m)} />
      {/each}
    </div>
  {/if}
</section>

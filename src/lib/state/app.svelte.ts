// Estado global de la UI. Se carga una vez y se mantiene con los eventos de Rust; los
// comandos devuelven además el estado nuevo, que se aplica en cuanto llega.
import {
  getBootStatus,
  getConfig,
  listProfiles,
  playProfile,
  stopProfile,
  updateConfig,
} from '../ipc/commands'
import { errorMessage } from '../ipc/errors'
import { onConfigChanged, onProfilesChanged, onRuntimeStatusChanged } from '../ipc/events'
import type {
  AppConfig,
  BootStatus,
  ConfigPatch,
  Profile,
  RuntimeStatus,
  SwapProgress,
} from '../ipc/types'
import { resolveLocale, setLocale } from '../i18n/index.svelte'

interface SwapState {
  action: 'play' | 'stop'
  profile: string | null
  active: boolean
  events: SwapProgress[]
  error: string | null
}

export const app = $state({
  ready: false,
  loadError: null as string | null,
  boot: null as BootStatus | null,
  config: null as AppConfig | null,
  profiles: [] as Profile[],
  runtime: { phase: 'idle', runningProfile: null, riotAlive: false } as RuntimeStatus,
  swap: null as SwapState | null,
})

export async function init(): Promise<void> {
  try {
    const [boot, config, profiles] = await Promise.all([
      getBootStatus(),
      getConfig(),
      listProfiles(),
    ])
    app.boot = boot
    app.config = config
    app.profiles = profiles
    app.runtime = boot.runtime
    await setLocale(resolveLocale(config.selectedLanguage))
    await onRuntimeStatusChanged((s) => (app.runtime = s))
    await onProfilesChanged((p) => (app.profiles = p))
    await onConfigChanged((c) => (app.config = c))
    app.ready = true
  } catch (e) {
    app.loadError = errorMessage(e)
  }
}

export async function refreshBoot(): Promise<void> {
  app.boot = await getBootStatus()
}

export async function patchConfig(patch: ConfigPatch): Promise<void> {
  app.config = await updateConfig(patch)
  if (patch.selectedLanguage !== undefined) {
    await setLocale(resolveLocale(app.config.selectedLanguage))
  }
}

async function runSwap(
  action: 'play' | 'stop',
  profile: string | null,
  call: (onProgress: (p: SwapProgress) => void) => Promise<RuntimeStatus>,
): Promise<void> {
  const swap: SwapState = { action, profile, active: true, events: [], error: null }
  app.swap = swap
  try {
    app.runtime = await call((p) => app.swap?.events.push(p))
  } catch (e) {
    if (app.swap) app.swap.error = errorMessage(e)
  } finally {
    if (app.swap) app.swap.active = false
  }
}

export const play = (name: string) =>
  runSwap('play', name, (onProgress) => playProfile(name, onProgress))

export const stop = () =>
  runSwap('stop', app.runtime.runningProfile, (onProgress) => stopProfile(onProgress))

export function dismissSwap(): void {
  if (!app.swap?.active) app.swap = null
}

// Suscripción tipada a los eventos de src-tauri/src/events.rs.
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AppConfig, Profile, RuntimeStatus } from './types'

export const onRuntimeStatusChanged = (cb: (s: RuntimeStatus) => void): Promise<UnlistenFn> =>
  listen<RuntimeStatus>('runtime-status-changed', (e) => cb(e.payload))

export const onProfilesChanged = (cb: (p: Profile[]) => void): Promise<UnlistenFn> =>
  listen<Profile[]>('profiles-changed', (e) => cb(e.payload))

export const onConfigChanged = (cb: (c: AppConfig) => void): Promise<UnlistenFn> =>
  listen<AppConfig>('config-changed', (e) => cb(e.payload))

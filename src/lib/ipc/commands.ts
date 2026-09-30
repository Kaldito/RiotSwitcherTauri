// Un wrapper tipado por comando de src-tauri/src/commands.
import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  AppConfig,
  BackgroundRef,
  BootStatus,
  ConfigPatch,
  NewProfileInput,
  Profile,
  ProfilePatch,
  RuntimeStatus,
  SwapProgress,
} from './types'

function channel<T>(onMessage: (message: T) => void): Channel<T> {
  const ch = new Channel<T>()
  ch.onmessage = onMessage
  return ch
}

export const getBootStatus = () => invoke<BootStatus>('get_boot_status')

export const setRiotClientLocation = (path: string) =>
  invoke<AppConfig>('set_riot_client_location', { path })

export const getConfig = () => invoke<AppConfig>('get_config')

export const updateConfig = (patch: ConfigPatch) => invoke<AppConfig>('update_config', { patch })

export const listProfiles = () => invoke<Profile[]>('list_profiles')

export const createProfile = (input: NewProfileInput) =>
  invoke<Profile>('create_profile', { input })

export const updateProfile = (name: string, patch: ProfilePatch) =>
  invoke<Profile>('update_profile', { name, patch })

export const deleteProfile = (name: string) => invoke<Profile[]>('delete_profile', { name })

export const reorderProfiles = (ordered: string[]) =>
  invoke<Profile[]>('reorder_profiles', { ordered })

export const importBackgroundImage = (sourcePath: string, nameHint: string | null) =>
  invoke<BackgroundRef>('import_background_image', { sourcePath, nameHint })

export const playProfile = (name: string, onProgress: (p: SwapProgress) => void) =>
  invoke<RuntimeStatus>('play_profile', { name, onProgress: channel(onProgress) })

export const stopProfile = (onProgress: (p: SwapProgress) => void) =>
  invoke<RuntimeStatus>('stop_profile', { onProgress: channel(onProgress) })

export const getRuntimeStatus = () => invoke<RuntimeStatus>('get_runtime_status')

export const quitApp = () => invoke<void>('quit_app')

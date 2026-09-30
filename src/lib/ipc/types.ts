// Reflejo de src-tauri/src/dto.rs y de los tipos IPC del core.

export type BackgroundRef = { kind: 'none' } | { kind: 'custom'; file: string }

export interface AppConfig {
  riotClientLocation: string
  selectedLanguage: string
  lastRunningProfile: string
  showAddProfileWarning: boolean
}

export interface ConfigPatch {
  selectedLanguage?: string
  showAddProfileWarning?: boolean
}

export interface Profile {
  name: string
  directoryName: string
  description: string
  background: BackgroundRef
  backgroundPath: string | null
}

export interface NewProfileInput {
  name: string | null
  description: string
  background: BackgroundRef
}

export interface ProfilePatch {
  name?: string | null
  description?: string | null
  background?: BackgroundRef | null
}

export type Phase = 'idle' | 'busy' | 'running'

export interface RuntimeStatus {
  phase: Phase
  runningProfile: string | null
  riotAlive: boolean
}

export interface BootStatus {
  riotClientLocation: string
  riotClientValid: boolean
  suggestedRiotClientLocation: string | null
  dataDir: string
  runtime: RuntimeStatus
}

export type SwapStep =
  | 'killing_processes'
  | 'waiting_processes'
  | 'saving_previous_session'
  | 'saving_session'
  | 'restoring_session'
  | 'launching'

export type SwapProgress =
  | { kind: 'step'; step: SwapStep; index: number; total: number }
  | { kind: 'warning'; code: string; path: string | null }
  | { kind: 'done' }

export interface AppError {
  code: string
  message: string
  details?: { path?: string }
}

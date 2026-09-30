import { t } from '../i18n/index.svelte'
import type { AppError } from './types'

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e
}

/** Texto traducido de `error.<code>`, o el mensaje en inglés si no hay traducción. */
export function errorMessage(e: unknown): string {
  if (!isAppError(e)) return String(e)
  const key = `error.${e.code}`
  const translated = t(key)
  const base = translated === key ? e.message : translated
  return e.details?.path ? `${base} (${e.details.path})` : base
}

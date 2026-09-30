// t() sobre runes. Cada idioma es una tabla `clave -> texto` en `<código>.json`; los
// errores del backend usan la clave `error.<code>`.
import en from './en_US.json'

type Table = Record<string, string>

export const LANGUAGES = [
  { code: 'en_US', label: 'English' },
  { code: 'es_ES', label: 'Español' },
] as const

const loaders: Record<string, () => Promise<{ default: Table }>> = {
  en_US: async () => ({ default: en }),
  es_ES: () => import('./es_ES.json'),
}

const i18n = $state({ locale: 'en_US', table: en as Table })

/** Traduce `key` y sustituye `%s` / `%d` en orden por `args`. */
export function t(key: string, ...args: (string | number)[]): string {
  let text = i18n.table[key] ?? (en as Table)[key] ?? key
  for (const arg of args) text = text.replace(/%[sd]/, String(arg))
  return text
}

export function currentLocale(): string {
  return i18n.locale
}

export async function setLocale(code: string): Promise<void> {
  const load = loaders[code] ?? loaders.en_US
  const table = (await load()).default
  i18n.locale = code in loaders ? code : 'en_US'
  i18n.table = table
  document.documentElement.lang = i18n.locale.replace('_', '-')
}

/** Idioma de la config; si está vacío, el del sistema si lo tenemos; si no, inglés. */
export function resolveLocale(configured: string): string {
  if (configured in loaders) return configured
  const system = navigator.language.toLowerCase()
  const match = LANGUAGES.find((l) => system.startsWith(l.code.slice(0, 2)))
  return match?.code ?? 'en_US'
}

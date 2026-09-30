import { convertFileSrc } from '@tauri-apps/api/core'
import type { BackgroundRef } from './ipc/types'

/** URL de la imagen de fondo, servida por el asset protocol; `null` si no tiene. */
export function backgroundSrc(bg: BackgroundRef, absolutePath: string | null): string | null {
  return bg.kind === 'custom' && absolutePath ? convertFileSrc(absolutePath) : null
}

/** Ruta absoluta de un fondo dentro de la carpeta de datos. */
export function customBackgroundPath(dataDir: string, file: string): string {
  return `${dataDir}\backgrounds\${file}`
}

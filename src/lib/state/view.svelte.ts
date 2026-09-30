// Vista actual. Una app de una ventana no necesita router.
export type ViewName = 'home' | 'form' | 'settings'

export const view = $state({
  name: 'home' as ViewName,
  /** Perfil que se edita en el formulario; `null` para crear uno nuevo. */
  editing: null as string | null,
})

export function goHome(): void {
  view.name = 'home'
  view.editing = null
}

export function openForm(editing: string | null = null): void {
  view.name = 'form'
  view.editing = editing
}

export function openSettings(): void {
  view.name = 'settings'
  view.editing = null
}

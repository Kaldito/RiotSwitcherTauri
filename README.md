<h1 align="center">RiotSwitcher</h1>

<p align="center">
  Cambia de cuenta de League of Legends con un clic.
</p>

<p align="center">
  <img alt="Versión" src="https://img.shields.io/badge/versión-0.7.0-blue">
  <img alt="Plataforma" src="https://img.shields.io/badge/plataforma-Windows-0078D6">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-24C8DB">
  <img alt="Svelte" src="https://img.shields.io/badge/Svelte-5-FF3E00">
  <img alt="Licencia" src="https://img.shields.io/badge/licencia-GPL--3.0-green">
</p>

---

> **La idea de este proyecto no es mía.** RiotSwitcher está inspirado en
> [**Riot Switcher**](https://github.com/arthiee4/RiotSwitcher), creado por
> [**arthiee4**](https://github.com/arthiee4) con Godot Engine. De ahí vienen la idea y
> el funcionamiento general; esta versión es una reimplementación independiente en
> Tauri v2 + Svelte 5. Todo el mérito de la idea original es suyo. Ver
> [Créditos](#créditos).

RiotSwitcher es una aplicación de escritorio para gestionar varias cuentas de League of
Legends. Guarda la sesión de cada cuenta en un perfil independiente y, al elegir un
perfil, abre el Riot Client con esa cuenta ya iniciada, sin volver a escribir usuario y
contraseña.

## Características

- **Perfiles por cuenta:** crea, edita, reordena y elimina perfiles, cada uno con su
  nombre, descripción e imagen de fondo opcional.
- **Cambio en un clic:** guarda la sesión de la cuenta activa, restaura la del perfil
  elegido y lanza el Riot Client.
- **Cierre seguro:** al salir se guarda la sesión en curso para no perderla.
- **Bandeja del sistema:** la aplicación sigue disponible desde el área de
  notificaciones.
- **Instancia única:** abrirla de nuevo enfoca la ventana existente.
- **Multidioma:** interfaz en inglés y español.

## Instalación

Descarga el instalador (`.exe` o `.msi`) desde la página de
[Releases](../../releases) y ejecútalo. El instalador `.exe` se instala sólo para el
usuario actual y no pide permisos de administrador.

Los datos de la aplicación se guardan en
`%LOCALAPPDATA%\com.kaldito.riotswitcher\`.

## Uso

1. En el primer arranque, selecciona la carpeta del Riot Client. La aplicación intenta
   detectarla automáticamente (por defecto, `C:\Riot Games\Riot Client`).
2. Crea un perfil y pulsa **Play**.
3. Inicia sesión en el Riot Client con la opción **"Mantener la sesión iniciada"**
   marcada.
4. Desde ese momento, pulsa **Play** en cualquier perfil para cambiar de cuenta.

> [!IMPORTANT]
> Sin la opción "Mantener la sesión iniciada" el Riot Client no conserva la sesión y el
> inicio automático no funciona.

## Compilar desde el código fuente

### Requisitos

- Windows 10 u 11
- [Rust](https://rustup.rs/) 1.90 o superior
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  con la carga de trabajo "Desarrollo para el escritorio con C++"
- [Node.js](https://nodejs.org/) 20 o superior y [pnpm](https://pnpm.io/)

### Comandos

```sh
pnpm install          # dependencias del frontend
pnpm tauri dev        # ejecutar en modo desarrollo
pnpm tauri build      # generar los instaladores NSIS y MSI
pnpm check            # comprobación de tipos del frontend
```

Los instaladores se generan en `src-tauri/target/release/bundle/`.

Para ejecutar los tests del núcleo:

```sh
cd src-tauri
cargo test -p riotswitcher-core
```

## Estructura del proyecto

```
├── src/                    Frontend (Svelte 5 + TypeScript)
│   └── lib/
│       ├── components/     Componentes de la interfaz
│       ├── views/          Pantallas de la aplicación
│       ├── state/          Estado global
│       ├── ipc/            Comunicación con el backend
│       └── i18n/           Traducciones
└── src-tauri/
    ├── src/                Aplicación Tauri: comandos, bandeja y orquestación
    └── crates/
        └── core/           Lógica de negocio independiente de Tauri
```

## Tecnologías

- [Tauri v2](https://tauri.app/) y [Rust](https://www.rust-lang.org/) en el backend
- [Svelte 5](https://svelte.dev/), [TypeScript](https://www.typescriptlang.org/) y
  [Vite](https://vite.dev/) en el frontend

## Créditos

- **[arthiee4](https://github.com/arthiee4)**, autor de
  [Riot Switcher](https://github.com/arthiee4/RiotSwitcher), el proyecto que inspiró
  este. La idea de cambiar de cuenta guardando y restaurando la sesión del Riot Client
  es suya.

## Licencia

Distribuido bajo la licencia GPL-3.0-or-later. Ver [LICENSE](LICENSE).

## Aviso legal

RiotSwitcher es un proyecto personal sin fines comerciales. No está afiliado, asociado,
autorizado ni respaldado por Riot Games, Inc. ni por ninguna de sus filiales. League of
Legends, Riot Games y Riot Client son marcas comerciales o registradas de Riot Games,
Inc.

El uso de esta aplicación es responsabilidad exclusiva del usuario. El autor no se hace
responsable de las consecuencias derivadas de su uso, incluidas sanciones o suspensiones
de cuentas.

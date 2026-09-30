//! Flujos Play, Stop y Quit. Son mutuamente excluyentes gracias a la
//! guardia `busy` y no se cancelan: una sesión a medio restaurar es peor que esperar.

pub mod play;
pub mod quit;
pub mod stop;

use std::path::Path;

use tauri::ipc::Channel;

use crate::dto::{SwapProgress, SwapStep};
use crate::error::AppError;

/// Envía el progreso por el canal del comando. Si la UI ya no escucha, se ignora.
pub(crate) struct Progress<'a> {
    channel: &'a Channel<SwapProgress>,
    index: u8,
    total: u8,
}

impl<'a> Progress<'a> {
    pub fn new(channel: &'a Channel<SwapProgress>, total: u8) -> Self {
        Progress {
            channel,
            index: 0,
            total,
        }
    }

    pub fn step(&mut self, step: SwapStep) {
        self.index += 1;
        log::info!("step {}/{}: {step:?}", self.index, self.total);
        let _ = self.channel.send(SwapProgress::Step {
            step,
            index: self.index,
            total: self.total,
        });
    }

    pub fn warn(&self, code: &'static str, path: Option<&Path>) {
        log::warn!("warning during swap: {code}");
        let _ = self.channel.send(SwapProgress::Warning {
            code,
            path: path.map(|p| p.display().to_string()),
        });
    }

    pub fn done(&self) {
        let _ = self.channel.send(SwapProgress::Done);
    }
}

pub(crate) fn join_error(e: tauri::Error) -> AppError {
    AppError::internal(format!("background task failed: {e}"))
}

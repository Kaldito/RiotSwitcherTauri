use std::path::{Path, PathBuf};

use crate::paths::LivePaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    File,
    Dir,
}

/// Raíz de la que cuelga la ruta viva de un ítem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveRoot {
    /// `%LOCALAPPDATA%\Riot Games\Riot Client`
    RiotLocalData,
    /// Carpeta de instalación del Riot Client.
    RiotClientLocation,
}

/// Un archivo o carpeta que forma parte de la sesión. Se guarda plano en
/// `profiles/<directory_name>/<name>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionItem {
    pub name: &'static str,
    pub kind: ItemKind,
    pub root: LiveRoot,
    /// Subcarpeta bajo la raíz donde vive el ítem.
    pub subdir: &'static str,
}

/// Versión de la tabla. Si Riot cambia los archivos de sesión se añade una tabla nueva
/// en lugar de repartir el cambio por el código.
pub const SESSION_ITEMS_VERSION: u32 = 1;

pub const SESSION_ITEMS: [SessionItem; 5] = [
    SessionItem {
        name: "RiotGamesPrivateSettings.yaml",
        kind: ItemKind::File,
        root: LiveRoot::RiotLocalData,
        subdir: "Data",
    },
    SessionItem {
        name: "Sessions",
        kind: ItemKind::Dir,
        root: LiveRoot::RiotLocalData,
        subdir: "Data",
    },
    SessionItem {
        name: "RiotClientSettings.yaml",
        kind: ItemKind::File,
        root: LiveRoot::RiotLocalData,
        subdir: "Config",
    },
    SessionItem {
        name: "client.config.yaml",
        kind: ItemKind::File,
        root: LiveRoot::RiotClientLocation,
        subdir: "Config",
    },
    SessionItem {
        name: "client.settings.yaml",
        kind: ItemKind::File,
        root: LiveRoot::RiotClientLocation,
        subdir: "Config",
    },
];

impl SessionItem {
    pub fn live_path(&self, live: &LivePaths) -> PathBuf {
        let root = match self.root {
            LiveRoot::RiotLocalData => &live.riot_local_data,
            LiveRoot::RiotClientLocation => &live.riot_client_location,
        };
        root.join(self.subdir).join(self.name)
    }

    pub fn stored_path(&self, profile_dir: &Path) -> PathBuf {
        profile_dir.join(self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_paths_match_the_spec() {
        let live = LivePaths::new(r"C:\L\Riot Games\Riot Client", r"C:\Riot Games\Riot Client");
        let paths: Vec<_> = SESSION_ITEMS.iter().map(|i| i.live_path(&live)).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from(r"C:\L\Riot Games\Riot Client\Data\RiotGamesPrivateSettings.yaml"),
                PathBuf::from(r"C:\L\Riot Games\Riot Client\Data\Sessions"),
                PathBuf::from(r"C:\L\Riot Games\Riot Client\Config\RiotClientSettings.yaml"),
                PathBuf::from(r"C:\Riot Games\Riot Client\Config\client.config.yaml"),
                PathBuf::from(r"C:\Riot Games\Riot Client\Config\client.settings.yaml"),
            ]
        );
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use super::items::{ItemKind, SESSION_ITEMS, SessionItem};
use crate::error::{CoreError, IoResultExt, Result};
use crate::fsutil::{copy_dir_replace, copy_file_replace, remove_dir_all_retry, remove_file_retry};
use crate::paths::LivePaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedItem {
    pub item: &'static str,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveReport {
    pub saved: Vec<&'static str>,
    /// Ítems que no existen en vivo; la copia anterior del perfil se conserva.
    pub missing: Vec<&'static str>,
    /// Ítems bloqueados tras agotar reintentos; la copia anterior del perfil se conserva.
    pub locked: Vec<LockedItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreReport {
    pub restored: Vec<&'static str>,
    /// Ítems que el perfil no tiene y que se borraron en vivo.
    pub cleared: Vec<&'static str>,
}

fn copy_item(item: &SessionItem, from: &Path, to: &Path) -> Result<()> {
    match item.kind {
        ItemKind::File => copy_file_replace(from, to).map(|_| ()),
        ItemKind::Dir => copy_dir_replace(from, to).map(|_| ()),
    }
}

fn item_exists(item: &SessionItem, path: &Path) -> bool {
    match item.kind {
        ItemKind::File => path.is_file(),
        ItemKind::Dir => path.is_dir(),
    }
}

/// Copia la sesión viva a la carpeta del perfil. Los ítems ausentes en vivo se omiten y
/// nunca se borra la copia anterior del perfil. Un bloqueo no aborta: se informa en
/// `locked` y se sigue con el resto. Cualquier otro error sí aborta.
pub fn save_session(live: &LivePaths, profile_dir: &Path) -> Result<SaveReport> {
    fs::create_dir_all(profile_dir).at(profile_dir)?;
    let mut report = SaveReport::default();
    for item in &SESSION_ITEMS {
        let from = item.live_path(live);
        if !item_exists(item, &from) {
            report.missing.push(item.name);
            continue;
        }
        match copy_item(item, &from, &item.stored_path(profile_dir)) {
            Ok(()) => report.saved.push(item.name),
            Err(CoreError::FileLocked(path)) => {
                log::warn!(
                    "session item {} is locked; keeping the previous copy",
                    item.name
                );
                report.locked.push(LockedItem {
                    item: item.name,
                    path,
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(report)
}

/// Copia la sesión del perfil a las rutas vivas. Si el perfil no tiene un ítem, se borra
/// el vivo para que el cliente abra en la pantalla de login. Cualquier fallo es fatal.
pub fn restore_session(live: &LivePaths, profile_dir: &Path) -> Result<RestoreReport> {
    let mut report = RestoreReport::default();
    for item in &SESSION_ITEMS {
        let stored = item.stored_path(profile_dir);
        let target = item.live_path(live);
        let result = if item_exists(item, &stored) {
            copy_item(item, &stored, &target).map(|()| report.restored.push(item.name))
        } else {
            let removal = match item.kind {
                ItemKind::File => remove_file_retry(&target),
                ItemKind::Dir => remove_dir_all_retry(&target),
            };
            removal.map(|()| report.cleared.push(item.name))
        };
        result.map_err(|e| CoreError::SessionRestoreFailed {
            item: item.name,
            source: Box::new(e),
        })?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _tmp: tempfile::TempDir,
        live: LivePaths,
        profile: PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let live = LivePaths::new(tmp.path().join("local"), tmp.path().join("client"));
        let profile = tmp.path().join("profiles").join("Main");
        Fixture {
            _tmp: tmp,
            live,
            profile,
        }
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn live(f: &Fixture, idx: usize) -> PathBuf {
        SESSION_ITEMS[idx].live_path(&f.live)
    }

    #[test]
    fn save_skips_missing_items() {
        let f = fixture();
        write(&live(&f, 0), "private-A");
        write(&live(&f, 1).join("s1.json"), "session-A");
        write(&live(&f, 2), "settings-A");
        let report = save_session(&f.live, &f.profile).unwrap();
        assert_eq!(
            report.saved,
            [
                "RiotGamesPrivateSettings.yaml",
                "Sessions",
                "RiotClientSettings.yaml"
            ]
        );
        assert_eq!(
            report.missing,
            ["client.config.yaml", "client.settings.yaml"]
        );
        assert_eq!(
            fs::read_to_string(f.profile.join("Sessions").join("s1.json")).unwrap(),
            "session-A"
        );
    }

    #[test]
    fn save_never_deletes_previous_copy() {
        let f = fixture();
        write(&f.profile.join("client.config.yaml"), "old");
        save_session(&f.live, &f.profile).unwrap();
        assert_eq!(
            fs::read_to_string(f.profile.join("client.config.yaml")).unwrap(),
            "old"
        );
    }

    #[test]
    fn restore_copies_and_clears() {
        let f = fixture();
        write(
            &f.profile.join("RiotGamesPrivateSettings.yaml"),
            "private-B",
        );
        write(&f.profile.join("Sessions").join("b.json"), "session-B");
        write(&live(&f, 0), "private-A");
        write(&live(&f, 1).join("a.json"), "session-A");
        write(&live(&f, 2), "settings-A");
        write(&live(&f, 3), "client-config-A");

        let report = restore_session(&f.live, &f.profile).unwrap();
        assert_eq!(
            report.restored,
            ["RiotGamesPrivateSettings.yaml", "Sessions"]
        );
        assert_eq!(
            report.cleared,
            [
                "RiotClientSettings.yaml",
                "client.config.yaml",
                "client.settings.yaml"
            ]
        );
        assert_eq!(fs::read_to_string(live(&f, 0)).unwrap(), "private-B");
        assert!(live(&f, 1).join("b.json").exists());
        assert!(
            !live(&f, 1).join("a.json").exists(),
            "Sessions is replaced whole"
        );
        assert!(!live(&f, 2).exists());
        assert!(!live(&f, 3).exists());
    }

    #[test]
    fn new_profile_restores_to_clean_login() {
        let f = fixture();
        fs::create_dir_all(&f.profile).unwrap();
        write(&live(&f, 0), "private-A");
        restore_session(&f.live, &f.profile).unwrap();
        assert!(!live(&f, 0).exists());
    }

    #[test]
    fn swap_roundtrip_between_two_profiles() {
        let f = fixture();
        let other = f.profile.parent().unwrap().join("Smurf");
        write(&live(&f, 0), "private-A");
        save_session(&f.live, &f.profile).unwrap();
        restore_session(&f.live, &other).unwrap();
        assert!(!live(&f, 0).exists());
        write(&live(&f, 0), "private-B");
        save_session(&f.live, &other).unwrap();
        restore_session(&f.live, &f.profile).unwrap();
        assert_eq!(fs::read_to_string(live(&f, 0)).unwrap(), "private-A");
    }

    #[cfg(windows)]
    #[test]
    fn locked_live_file_is_a_warning_on_save_and_fatal_on_restore() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = fixture();
        write(&live(&f, 0), "private-A");
        write(&live(&f, 2), "settings-A");
        write(&f.profile.join("RiotGamesPrivateSettings.yaml"), "previous");
        let _guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(live(&f, 0))
            .unwrap();

        let report = save_session(&f.live, &f.profile).unwrap();
        assert_eq!(report.locked.len(), 1);
        assert_eq!(report.saved, ["RiotClientSettings.yaml"]);
        assert_eq!(
            fs::read_to_string(f.profile.join("RiotGamesPrivateSettings.yaml")).unwrap(),
            "previous"
        );

        let err = restore_session(&f.live, &f.profile).unwrap_err();
        assert_eq!(err.code(), "session_restore_failed");
    }
}

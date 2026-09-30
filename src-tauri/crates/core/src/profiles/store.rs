use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::model::{BackgroundRef, NewProfile, Profile, ProfilePatch};
use super::sanitize::{MAX_PROFILE_NAME_LEN, resolve_unique, sanitize_directory_name};
use crate::error::{CoreError, IoResultExt, Result};
use crate::fsutil::{read_json_lenient, remove_dir_all_retry, with_retry, write_json_atomic};
use crate::paths::AppDirs;

pub const PROFILES_SCHEMA_VERSION: u32 = 1;

/// Contenido de `data/profiles_data.json`. El orden del vector es el orden visual.
///
/// Las operaciones que cambian datos trabajan sobre una copia, la guardan en disco y sólo
/// entonces la adoptan: si el guardado falla, la base en memoria no cambia.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileDb {
    pub profiles: Vec<Profile>,
}

#[derive(Serialize)]
struct ProfilesFile<'a> {
    schema_version: u32,
    profiles: &'a [Profile],
}

fn validate_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(CoreError::invalid("the profile name cannot be empty"));
    }
    if name.chars().count() > MAX_PROFILE_NAME_LEN {
        return Err(CoreError::invalid(format!(
            "the profile name cannot exceed {MAX_PROFILE_NAME_LEN} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(CoreError::invalid(
            "the profile name contains control characters",
        ));
    }
    Ok(name.to_string())
}

impl ProfileDb {
    /// Carga el archivo. Si falta o está corrupto, devuelve una base vacía.
    pub fn load(path: &Path) -> Result<ProfileDb> {
        let value: Option<Value> = read_json_lenient(path)?;
        Ok(value
            .map(|v| Self::from_value(&v, crate::now_unix()))
            .unwrap_or_default())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_json_atomic(
            path,
            &ProfilesFile {
                schema_version: PROFILES_SCHEMA_VERSION,
                profiles: &self.profiles,
            },
        )
    }

    /// Interpreta el JSON del archivo. Las entradas ilegibles se descartan.
    pub fn from_value(value: &Value, now_unix: u64) -> ProfileDb {
        let profiles = value
            .get("profiles")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(
                        |item| match serde_json::from_value::<Profile>(item.clone()) {
                            Ok(p) => Some(p),
                            Err(e) => {
                                log::warn!("skipping unreadable profile entry: {e}");
                                None
                            }
                        },
                    )
                    .collect()
            })
            .unwrap_or_default();
        ProfileDb {
            profiles: normalize(profiles, now_unix),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.profile_name == name)
    }

    fn position(&self, name: &str) -> Option<usize> {
        self.profiles.iter().position(|p| p.profile_name == name)
    }

    fn name_taken(&self, name: &str, except: Option<usize>) -> bool {
        self.profiles
            .iter()
            .enumerate()
            .any(|(i, p)| Some(i) != except && p.profile_name.to_lowercase() == name.to_lowercase())
    }

    fn dir_taken(&self, dirs: &AppDirs, candidate: &str, except: Option<usize>) -> bool {
        if let Some(i) = except
            && self.profiles[i]
                .directory_name
                .eq_ignore_ascii_case(candidate)
        {
            return false;
        }
        self.profiles
            .iter()
            .enumerate()
            .any(|(i, p)| Some(i) != except && p.directory_name.eq_ignore_ascii_case(candidate))
            || dirs.profiles.join(candidate).exists()
    }

    fn auto_name(&self) -> String {
        (1u32..)
            .map(|n| format!("Profile {n}"))
            .find(|c| !self.name_taken(c, None))
            .expect("an unbounded range always yields a free name")
    }

    fn commit(&mut self, next: ProfileDb, dirs: &AppDirs) -> Result<()> {
        next.save(&dirs.profiles_file())?;
        *self = next;
        Ok(())
    }

    /// Crea un perfil y su carpeta.
    pub fn create(&mut self, dirs: &AppDirs, input: NewProfile, now_unix: u64) -> Result<Profile> {
        let typed = input
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        let name = match typed {
            Some(n) => validate_name(n)?,
            None => self.auto_name(),
        };
        if self.name_taken(&name, None) {
            return Err(CoreError::ProfileNameTaken(name));
        }
        if !input.background.is_valid() {
            return Err(CoreError::invalid("invalid background"));
        }

        let base = sanitize_directory_name(&name, now_unix);
        let directory_name = resolve_unique(&base, |c| self.dir_taken(dirs, c, None));
        let dir = dirs.profile_dir_checked(&directory_name)?;
        fs::create_dir_all(&dir).at(&dir)?;

        let profile = Profile {
            profile_name: name,
            directory_name,
            description: input.description.trim().to_string(),
            background: input.background,
            ..Profile::default()
        };
        let mut next = self.clone();
        next.profiles.push(profile.clone());
        if let Err(e) = self.commit(next, dirs) {
            let _ = fs::remove_dir(&dir);
            return Err(e);
        }
        Ok(profile)
    }

    /// Cambia nombre, descripción o fondo. Renombrar mueve la carpeta del perfil y se
    /// rechaza si el perfil está en ejecución.
    pub fn update(
        &mut self,
        dirs: &AppDirs,
        name: &str,
        patch: ProfilePatch,
        running: Option<&str>,
        now_unix: u64,
    ) -> Result<Profile> {
        let idx = self
            .position(name)
            .ok_or_else(|| CoreError::ProfileNotFound(name.to_string()))?;
        let mut next = self.clone();
        let mut moved: Option<(PathBuf, PathBuf)> = None;

        if let Some(raw) = patch.name.as_deref() {
            let new_name = validate_name(raw)?;
            let current = &next.profiles[idx];
            if new_name != current.profile_name {
                if running == Some(current.profile_name.as_str()) {
                    return Err(CoreError::ProfileRunning(current.profile_name.clone()));
                }
                if self.name_taken(&new_name, Some(idx)) {
                    return Err(CoreError::ProfileNameTaken(new_name));
                }
                let base = sanitize_directory_name(&new_name, now_unix);
                let new_dir = if base.eq_ignore_ascii_case(&current.directory_name) {
                    current.directory_name.clone()
                } else {
                    resolve_unique(&base, |c| self.dir_taken(dirs, c, Some(idx)))
                };
                if new_dir != current.directory_name {
                    let from = dirs.profile_dir_checked(&current.directory_name)?;
                    let to = dirs.profile_dir_checked(&new_dir)?;
                    if from.exists() {
                        with_retry(&from, || fs::rename(&from, &to))?;
                        moved = Some((from, to));
                    } else {
                        fs::create_dir_all(&to).at(&to)?;
                    }
                }
                let p = &mut next.profiles[idx];
                p.directory_name = new_dir;
                p.profile_name = new_name;
            }
        }

        let p = &mut next.profiles[idx];
        if let Some(desc) = patch.description {
            p.description = desc.trim().to_string();
        }
        if let Some(bg) = patch.background {
            if !bg.is_valid() {
                return Err(CoreError::invalid("invalid background"));
            }
            p.background = bg;
        }
        let updated = p.clone();

        if let Err(e) = self.commit(next, dirs) {
            if let Some((from, to)) = moved {
                let _ = fs::rename(&to, &from);
            }
            return Err(e);
        }
        Ok(updated)
    }

    /// Borra el perfil y, tras guardar, su carpeta.
    pub fn delete(&mut self, dirs: &AppDirs, name: &str, running: Option<&str>) -> Result<()> {
        let idx = self
            .position(name)
            .ok_or_else(|| CoreError::ProfileNotFound(name.to_string()))?;
        if running == Some(name) {
            return Err(CoreError::ProfileRunning(name.to_string()));
        }
        let mut next = self.clone();
        let removed = next.profiles.remove(idx);
        self.commit(next, dirs)?;
        match dirs.profile_dir_checked(&removed.directory_name) {
            Ok(dir) => {
                if let Err(e) = remove_dir_all_retry(&dir) {
                    log::warn!("profile deleted but its folder could not be removed: {e}");
                }
            }
            Err(e) => log::warn!("profile deleted with an invalid folder name: {e}"),
        }
        Ok(())
    }

    /// Reordena según `ordered`. Los nombres desconocidos se ignoran y los perfiles que
    /// falten en la lista conservan su orden relativo al final.
    pub fn reorder(&mut self, dirs: &AppDirs, ordered: &[String]) -> Result<()> {
        let mut result: Vec<Profile> = Vec::with_capacity(self.profiles.len());
        for name in ordered {
            if let Some(p) = self.get(name)
                && !result.iter().any(|r| &r.profile_name == name)
            {
                result.push(p.clone());
            }
        }
        for p in &self.profiles {
            if !result.iter().any(|r| r.profile_name == p.profile_name) {
                result.push(p.clone());
            }
        }
        self.commit(ProfileDb { profiles: result }, dirs)
    }
}

/// Quita perfiles sin nombre o con nombre repetido y garantiza carpetas válidas y únicas.
fn normalize(profiles: Vec<Profile>, now_unix: u64) -> Vec<Profile> {
    let mut out: Vec<Profile> = Vec::with_capacity(profiles.len());
    for mut p in profiles {
        p.profile_name = p.profile_name.trim().to_string();
        if p.profile_name.is_empty() {
            log::warn!("skipping a profile without name");
            continue;
        }
        if out.iter().any(|o| o.profile_name == p.profile_name) {
            log::warn!("skipping a duplicated profile entry");
            continue;
        }
        if !AppDirs::is_valid_dir_name(&p.directory_name) {
            p.directory_name = sanitize_directory_name(&p.profile_name, now_unix);
        }
        if out
            .iter()
            .any(|o| o.directory_name.eq_ignore_ascii_case(&p.directory_name))
        {
            let base = p.directory_name.clone();
            p.directory_name = resolve_unique(&base, |c| {
                out.iter().any(|o| o.directory_name.eq_ignore_ascii_case(c))
            });
            log::warn!("profile folder name collision resolved");
        }
        if !p.background.is_valid() {
            p.background = BackgroundRef::None;
        }
        out.push(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup() -> (tempfile::TempDir, AppDirs) {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = AppDirs::new(tmp.path().join("app"));
        dirs.ensure().unwrap();
        (tmp, dirs)
    }

    fn named(name: &str) -> NewProfile {
        NewProfile {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    #[test]
    fn create_persists_and_creates_folder() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        let p = db.create(&dirs, named("My Main"), 1).unwrap();
        assert_eq!(p.directory_name, "My_Main");
        assert!(dirs.profile_dir("My_Main").is_dir());
        assert_eq!(ProfileDb::load(&dirs.profiles_file()).unwrap(), db);
    }

    #[test]
    fn create_without_name_uses_auto_name() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        let a = db.create(&dirs, NewProfile::default(), 1).unwrap();
        let b = db.create(&dirs, named("  "), 1).unwrap();
        assert_eq!(a.profile_name, "Profile 1");
        assert_eq!(b.profile_name, "Profile 2");
    }

    #[test]
    fn create_rejects_duplicate_names_case_insensitive() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        db.create(&dirs, named("Main"), 1).unwrap();
        let err = db.create(&dirs, named("main"), 1).unwrap_err();
        assert_eq!(err.code(), "profile_name_taken");
    }

    #[test]
    fn create_avoids_existing_folders() {
        let (_tmp, dirs) = setup();
        fs::create_dir_all(dirs.profile_dir("Main")).unwrap();
        let mut db = ProfileDb::default();
        let p = db.create(&dirs, named("Main"), 1).unwrap();
        assert_eq!(p.directory_name, "Main_2");
    }

    #[test]
    fn rename_moves_folder() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        db.create(&dirs, named("Main"), 1).unwrap();
        fs::write(
            dirs.profile_dir("Main").join("RiotClientSettings.yaml"),
            b"x",
        )
        .unwrap();
        let patch = ProfilePatch {
            name: Some("Smurf Acc".into()),
            description: Some("  alt  ".into()),
            background: Some(BackgroundRef::Custom {
                file: "bg.png".into(),
            }),
        };
        let p = db.update(&dirs, "Main", patch, None, 1).unwrap();
        assert_eq!(p.profile_name, "Smurf Acc");
        assert_eq!(p.directory_name, "Smurf_Acc");
        assert_eq!(p.description, "alt");
        assert!(
            dirs.profile_dir("Smurf_Acc")
                .join("RiotClientSettings.yaml")
                .exists()
        );
        assert!(!dirs.profile_dir("Main").exists());
        assert!(db.get("Main").is_none());
    }

    #[test]
    fn rename_of_running_profile_is_rejected() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        db.create(&dirs, named("Main"), 1).unwrap();
        let patch = ProfilePatch {
            name: Some("Other".into()),
            ..Default::default()
        };
        let err = db
            .update(&dirs, "Main", patch, Some("Main"), 1)
            .unwrap_err();
        assert_eq!(err.code(), "profile_running");
        let desc_only = ProfilePatch {
            description: Some("ok".into()),
            ..Default::default()
        };
        db.update(&dirs, "Main", desc_only, Some("Main"), 1)
            .unwrap();
    }

    #[test]
    fn delete_removes_folder_and_respects_running() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        db.create(&dirs, named("A"), 1).unwrap();
        db.create(&dirs, named("B"), 1).unwrap();
        assert_eq!(
            db.delete(&dirs, "A", Some("A")).unwrap_err().code(),
            "profile_running"
        );
        db.delete(&dirs, "A", Some("B")).unwrap();
        assert!(!dirs.profile_dir("A").exists());
        assert!(dirs.profile_dir("B").exists());
        assert_eq!(
            db.delete(&dirs, "A", None).unwrap_err().code(),
            "profile_not_found"
        );
    }

    #[test]
    fn reorder_keeps_missing_at_the_end() {
        let (_tmp, dirs) = setup();
        let mut db = ProfileDb::default();
        for n in ["A", "B", "C"] {
            db.create(&dirs, named(n), 1).unwrap();
        }
        db.reorder(&dirs, &["C".into(), "ghost".into(), "A".into(), "C".into()])
            .unwrap();
        let names: Vec<_> = db
            .profiles
            .iter()
            .map(|p| p.profile_name.as_str())
            .collect();
        assert_eq!(names, ["C", "A", "B"]);
    }

    #[test]
    fn roundtrip_keeps_unknown_keys() {
        let (_tmp, dirs) = setup();
        let raw = json!({"schema_version": 1, "profiles": [
            {"profile_name": "A", "directory_name": "A", "background": {"kind": "default"}, "future": 1}
        ], "top_level_future": true});
        fs::write(dirs.profiles_file(), serde_json::to_vec(&raw).unwrap()).unwrap();
        let db = ProfileDb::load(&dirs.profiles_file()).unwrap();
        db.save(&dirs.profiles_file()).unwrap();
        let back: Value = serde_json::from_slice(&fs::read(dirs.profiles_file()).unwrap()).unwrap();
        assert_eq!(back["schema_version"], 1);
        assert_eq!(back["profiles"][0]["future"], 1);
    }

    #[test]
    fn duplicated_folders_are_disambiguated() {
        let raw = json!({"schema_version": 1, "profiles": [
            {"profile_name": "A", "directory_name": "same"},
            {"profile_name": "B", "directory_name": "SAME"},
            {"profile_name": "C", "directory_name": ".."}
        ]});
        let db = ProfileDb::from_value(&raw, 1);
        let dirs: Vec<_> = db
            .profiles
            .iter()
            .map(|p| p.directory_name.as_str())
            .collect();
        assert_eq!(dirs, ["same", "SAME_2", "C"]);
    }

    #[test]
    fn corrupt_file_gives_empty_db() {
        let (_tmp, dirs) = setup();
        fs::write(dirs.profiles_file(), b"garbage").unwrap();
        let db = ProfileDb::load(&dirs.profiles_file()).unwrap();
        assert!(db.profiles.is_empty());
        assert!(dirs.data.join("profiles_data.json.corrupt.bak").exists());
    }
}

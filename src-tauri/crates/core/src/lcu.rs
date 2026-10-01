//! API local del cliente de League (LCU): sólo lo necesario para leer el icono de
//! invocador de la cuenta que tiene la sesión abierta. El cliente escucha en
//! `127.0.0.1` con un certificado autofirmado y un token que cambia en cada arranque.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use ureq::tls::TlsConfig;

use crate::error::{CoreError, Result};

const LEAGUE_CLIENT_EXE: &str = "LeagueClient.exe";
const LEAGUE_CLIENT_UX_EXE: &str = "LeagueClientUx.exe";
const LOCKFILE: &str = "lockfile";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcuCredentials {
    pub port: u16,
    pub token: String,
}

/// Lee `LeagueClient:<pid>:<puerto>:<token>:https`.
pub fn parse_lockfile(content: &str) -> Option<LcuCredentials> {
    let mut parts = content.trim().split(':');
    let port = parts.nth(2)?.parse().ok()?;
    let token = parts.next().filter(|t| !t.is_empty())?.to_string();
    Some(LcuCredentials { port, token })
}

/// Lee `--app-port=` y `--remoting-auth-token=` de los argumentos de `LeagueClientUx.exe`.
pub fn parse_cmd_args<'a>(args: impl IntoIterator<Item = &'a str>) -> Option<LcuCredentials> {
    let mut port = None;
    let mut token = None;
    for arg in args {
        let arg = arg.trim_matches('"');
        if let Some(v) = arg.strip_prefix("--app-port=") {
            port = v.parse().ok();
        } else if let Some(v) = arg.strip_prefix("--remoting-auth-token=") {
            token = Some(v.to_string()).filter(|t| !t.is_empty());
        }
    }
    Some(LcuCredentials {
        port: port?,
        token: token?,
    })
}

/// Busca el cliente de League en ejecución: primero el `lockfile` junto a
/// `LeagueClient.exe` y, si no, los argumentos de `LeagueClientUx.exe`.
fn find_credentials() -> Option<LcuCredentials> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );
    let named = |name: &'static str| {
        sys.processes()
            .values()
            .filter(move |p| p.name().eq_ignore_ascii_case(name))
    };
    named(LEAGUE_CLIENT_EXE)
        .find_map(|p| {
            let lockfile = p.exe()?.parent()?.join(LOCKFILE);
            parse_lockfile(&std::fs::read_to_string(lockfile).ok()?)
        })
        .or_else(|| {
            named(LEAGUE_CLIENT_UX_EXE).find_map(|p| {
                let args: Vec<String> = p
                    .cmd()
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect();
                parse_cmd_args(args.iter().map(String::as_str))
            })
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurrentSummoner {
    profile_icon_id: u32,
}

fn get(agent: &ureq::Agent, creds: &LcuCredentials, path: &str) -> Result<Vec<u8>> {
    let auth = STANDARD.encode(format!("riot:{}", creds.token));
    agent
        .get(format!("https://127.0.0.1:{}{path}", creds.port))
        .header("Authorization", format!("Basic {auth}"))
        .call()
        .and_then(|mut resp| resp.body_mut().read_to_vec())
        .map_err(|e| CoreError::LeagueClientRequestFailed(format!("{path}: {e}")))
}

/// Descarga del cliente de League el icono de invocador (JPG) de la cuenta con la sesión
/// abierta.
pub fn fetch_profile_icon() -> Result<Vec<u8>> {
    let creds = find_credentials().ok_or(CoreError::LeagueClientNotRunning)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().disable_verification(true).build())
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into();
    let body = get(&agent, &creds, "/lol-summoner/v1/current-summoner")?;
    let summoner: CurrentSummoner = serde_json::from_slice(&body)
        .map_err(|e| CoreError::LeagueClientRequestFailed(e.to_string()))?;
    let icon = get(
        &agent,
        &creds,
        &format!(
            "/lol-game-data/assets/v1/profile-icons/{}.jpg",
            summoner.profile_icon_id
        ),
    )?;
    if icon.is_empty() {
        return Err(CoreError::LeagueClientRequestFailed(
            "empty profile icon".into(),
        ));
    }
    Ok(icon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lockfile() {
        assert_eq!(
            parse_lockfile("LeagueClient:1234:50123:abcDEF:https\n"),
            Some(LcuCredentials {
                port: 50123,
                token: "abcDEF".into()
            })
        );
        assert_eq!(parse_lockfile("LeagueClient:1234"), None);
        assert_eq!(parse_lockfile(""), None);
    }

    #[test]
    fn parses_cmd_args() {
        let args = [
            "LeagueClientUx.exe",
            "\"--remoting-auth-token=tok\"",
            "--app-port=4567",
        ];
        assert_eq!(
            parse_cmd_args(args),
            Some(LcuCredentials {
                port: 4567,
                token: "tok".into()
            })
        );
        assert_eq!(parse_cmd_args(["--app-port=4567"]), None);
    }
}

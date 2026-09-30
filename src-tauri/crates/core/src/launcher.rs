//! Lanzamiento del Riot Client con League of Legends.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{CoreError, Result};
use crate::paths::{RIOT_CLIENT_EXE, validate_riot_client_location};

pub const LAUNCH_ARGS: [&str; 2] = [
    "--launch-product=league_of_legends",
    "--launch-patchline=live",
];

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// `<riot_dir>\RiotClientServices.exe --launch-product=league_of_legends
/// --launch-patchline=live`, separado del grupo de procesos de la app y sin heredar la
/// consola.
pub fn build_launch_command(riot_dir: &Path) -> Command {
    let mut cmd = Command::new(riot_dir.join(RIOT_CLIENT_EXE));
    cmd.args(LAUNCH_ARGS)
        .current_dir(riot_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    cmd
}

/// Lanza el cliente y devuelve su PID. No espera a que termine.
pub fn launch(riot_dir: &Path) -> Result<u32> {
    validate_riot_client_location(riot_dir)?;
    let child = build_launch_command(riot_dir)
        .spawn()
        .map_err(CoreError::LaunchFailed)?;
    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn builds_expected_arguments() {
        let cmd = build_launch_command(Path::new(r"C:\Riot Games\Riot Client"));
        assert_eq!(
            cmd.get_program(),
            OsStr::new(r"C:\Riot Games\Riot Client\RiotClientServices.exe")
        );
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, LAUNCH_ARGS.map(OsStr::new));
        assert_eq!(
            cmd.get_current_dir(),
            Some(Path::new(r"C:\Riot Games\Riot Client"))
        );
    }

    #[test]
    fn launch_requires_the_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let err = launch(tmp.path()).unwrap_err();
        assert_eq!(err.code(), "riot_client_not_found");
    }
}

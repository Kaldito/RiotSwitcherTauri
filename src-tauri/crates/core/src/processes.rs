//! Procesos del Riot Client y de League: listar, matar y esperar a que mueran.
//! `ProcessProbe` permite sustituir `sysinfo` por un simulador en los tests.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::timing::{PROCESS_KILL_TIMEOUT, PROCESS_POLL_INTERVAL, PROCESS_REKILL_INTERVAL};

/// Procesos que se cierran antes de cada cambio de perfil: el Riot Client (incluidos los
/// nombres de versiones anteriores), el cliente de League, el juego y sus crash handlers.
pub const RIOT_PROCESS_NAMES: [&str; 10] = [
    "RiotClientServices.exe",
    "Riot Client.exe",
    "RiotClientUx.exe",
    "RiotClientUxRender.exe",
    "RiotClientCrashHandler.exe",
    "LeagueClient.exe",
    "LeagueClientUx.exe",
    "LeagueClientUxRender.exe",
    "LeagueofLegends.exe",
    "LeagueCrashHandler64.exe",
];

/// Procesos cuya presencia al arrancar indica que el último perfil sigue en uso.
pub const READOPT_PROCESS_NAMES: [&str; 2] = ["RiotClientServices.exe", "LeagueClient.exe"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcInfo {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    /// Segundos desde la época; sirve para descartar padres cuyo PID se ha reutilizado.
    pub start_time: u64,
}

pub trait ProcessProbe {
    fn snapshot(&mut self) -> Vec<ProcInfo>;
    fn kill(&mut self, pid: u32) -> bool;
}

/// Implementación real sobre `sysinfo`, refrescando sólo lo imprescindible.
pub struct SysinfoProbe {
    sys: System,
}

impl SysinfoProbe {
    pub fn new() -> Self {
        SysinfoProbe { sys: System::new() }
    }
}

impl Default for SysinfoProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessProbe for SysinfoProbe {
    fn snapshot(&mut self) -> Vec<ProcInfo> {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        self.sys
            .processes()
            .iter()
            .map(|(pid, p)| ProcInfo {
                pid: pid.as_u32(),
                parent: p.parent().map(|pp| pp.as_u32()),
                name: p.name().to_string_lossy().into_owned(),
                start_time: p.start_time(),
            })
            .collect()
    }

    fn kill(&mut self, pid: u32) -> bool {
        self.sys
            .process(Pid::from_u32(pid))
            .map(|p| p.kill())
            .unwrap_or(false)
    }
}

/// Reloj inyectable para probar las esperas sin dormir de verdad.
pub trait Clock {
    fn elapsed(&self) -> Duration;
    fn sleep(&mut self, d: Duration);
}

pub struct RealClock {
    start: Instant,
}

impl RealClock {
    pub fn new() -> Self {
        RealClock {
            start: Instant::now(),
        }
    }
}

impl Default for RealClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for RealClock {
    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

fn name_matches(name: &str, names: &[&str]) -> bool {
    names.iter().any(|n| n.eq_ignore_ascii_case(name))
}

fn is_protected(pid: u32) -> bool {
    pid <= 4 || pid == std::process::id()
}

/// Procesos cuyo nombre está en `names`, sin descendientes.
pub fn find_by_name(procs: &[ProcInfo], names: &[&str]) -> Vec<ProcInfo> {
    procs
        .iter()
        .filter(|p| !is_protected(p.pid) && name_matches(&p.name, names))
        .cloned()
        .collect()
}

/// Procesos cuyo nombre está en `names` más todos sus descendientes. Un hijo sólo cuenta
/// si arrancó después que su padre, para no seguir PIDs reutilizados.
pub fn select_targets(procs: &[ProcInfo], names: &[&str]) -> Vec<ProcInfo> {
    let mut selected: HashSet<u32> = find_by_name(procs, names).iter().map(|p| p.pid).collect();
    let start_of = |pid: u32| procs.iter().find(|p| p.pid == pid).map(|p| p.start_time);
    loop {
        let before = selected.len();
        for p in procs {
            if selected.contains(&p.pid) || is_protected(p.pid) {
                continue;
            }
            if let Some(parent) = p.parent
                && parent != p.pid
                && selected.contains(&parent)
                && start_of(parent).is_some_and(|ps| p.start_time >= ps)
            {
                selected.insert(p.pid);
            }
        }
        if selected.len() == before {
            break;
        }
    }
    procs
        .iter()
        .filter(|p| selected.contains(&p.pid))
        .cloned()
        .collect()
}

/// Mata los procesos de `names` y sus descendientes. Devuelve los que se intentó matar.
pub fn kill_all(probe: &mut dyn ProcessProbe, names: &[&str]) -> Vec<ProcInfo> {
    let targets = select_targets(&probe.snapshot(), names);
    for p in &targets {
        if !probe.kill(p.pid) {
            log::debug!("kill request failed for {} ({})", p.name, p.pid);
        }
    }
    if !targets.is_empty() {
        log::info!("sent kill to {} Riot process(es)", targets.len());
    }
    targets
}

pub fn is_any_alive(probe: &mut dyn ProcessProbe, names: &[&str]) -> bool {
    !find_by_name(&probe.snapshot(), names).is_empty()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitOutcome {
    pub all_dead: bool,
    /// Nombres de los procesos que seguían vivos al agotar el tiempo.
    pub remaining: Vec<String>,
    pub rekills: u32,
}

/// Espera a que no quede ningún proceso de `names` ni descendiente. Sondea cada 250 ms,
/// repite el kill cada 2 s y se rinde a los 6 s. El tiempo se mide con el reloj,
/// no sumando esperas.
pub fn wait_until_all_dead(
    probe: &mut dyn ProcessProbe,
    names: &[&str],
    clock: &mut dyn Clock,
) -> WaitOutcome {
    let start = clock.elapsed();
    let mut last_kill = start;
    let mut rekills = 0;
    loop {
        let alive = select_targets(&probe.snapshot(), names);
        if alive.is_empty() {
            return WaitOutcome {
                all_dead: true,
                remaining: Vec::new(),
                rekills,
            };
        }
        let now = clock.elapsed();
        if now - start >= PROCESS_KILL_TIMEOUT {
            let mut remaining: Vec<String> = alive.into_iter().map(|p| p.name).collect();
            remaining.sort();
            remaining.dedup();
            log::warn!("Riot processes still alive after timeout: {remaining:?}");
            return WaitOutcome {
                all_dead: false,
                remaining,
                rekills,
            };
        }
        if now - last_kill >= PROCESS_REKILL_INTERVAL {
            for p in &alive {
                probe.kill(p.pid);
            }
            rekills += 1;
            last_kill = now;
        }
        clock.sleep(PROCESS_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent: Option<u32>, name: &str, start: u64) -> ProcInfo {
        ProcInfo {
            pid,
            parent,
            name: name.into(),
            start_time: start,
        }
    }

    struct FakeClock {
        now: Duration,
    }

    impl Clock for FakeClock {
        fn elapsed(&self) -> Duration {
            self.now
        }
        fn sleep(&mut self, d: Duration) {
            self.now += d;
        }
    }

    /// Simulador: cada PID muere tras recibir `kills_to_die` kills.
    struct FakeProbe {
        procs: Vec<(ProcInfo, u32)>,
        kills: Vec<u32>,
    }

    impl ProcessProbe for FakeProbe {
        fn snapshot(&mut self) -> Vec<ProcInfo> {
            self.procs.iter().map(|(p, _)| p.clone()).collect()
        }
        fn kill(&mut self, pid: u32) -> bool {
            self.kills.push(pid);
            for (p, remaining) in &mut self.procs {
                if p.pid == pid {
                    *remaining = remaining.saturating_sub(1);
                }
            }
            self.procs.retain(|(_, remaining)| *remaining > 0);
            true
        }
    }

    #[test]
    fn selects_by_name_and_descendants() {
        let procs = vec![
            proc(100, Some(1), "RiotClientServices.exe", 10),
            proc(101, Some(100), "Riot Client.exe", 11),
            proc(102, Some(101), "SomeHelper.exe", 12),
            proc(200, Some(1), "explorer.exe", 1),
            proc(300, Some(100), "reused-pid-child.exe", 5),
            proc(400, Some(1), "leagueclient.EXE", 20),
        ];
        let mut pids: Vec<u32> = select_targets(&procs, &RIOT_PROCESS_NAMES)
            .iter()
            .map(|p| p.pid)
            .collect();
        pids.sort();
        assert_eq!(pids, [100, 101, 102, 400]);
    }

    #[test]
    fn never_targets_own_process() {
        let me = std::process::id();
        let procs = vec![proc(me, None, "RiotClientServices.exe", 1)];
        assert!(select_targets(&procs, &RIOT_PROCESS_NAMES).is_empty());
    }

    #[test]
    fn wait_returns_immediately_when_nothing_runs() {
        let mut probe = FakeProbe {
            procs: vec![],
            kills: vec![],
        };
        let mut clock = FakeClock {
            now: Duration::ZERO,
        };
        let out = wait_until_all_dead(&mut probe, &RIOT_PROCESS_NAMES, &mut clock);
        assert!(out.all_dead);
        assert_eq!(clock.now, Duration::ZERO);
    }

    #[test]
    fn stubborn_process_gets_rekilled() {
        let mut probe = FakeProbe {
            procs: vec![(proc(100, None, "LeagueClient.exe", 1), 2)],
            kills: vec![],
        };
        kill_all(&mut probe, &RIOT_PROCESS_NAMES);
        let mut clock = FakeClock {
            now: Duration::ZERO,
        };
        let out = wait_until_all_dead(&mut probe, &RIOT_PROCESS_NAMES, &mut clock);
        assert!(out.all_dead);
        assert_eq!(out.rekills, 1);
        assert_eq!(probe.kills, [100, 100]);
        assert_eq!(clock.now, Duration::from_millis(2250));
    }

    #[test]
    fn times_out_after_six_seconds() {
        let mut probe = FakeProbe {
            procs: vec![(proc(100, None, "LeagueofLegends.exe", 1), u32::MAX)],
            kills: vec![],
        };
        let mut clock = FakeClock {
            now: Duration::ZERO,
        };
        let out = wait_until_all_dead(&mut probe, &RIOT_PROCESS_NAMES, &mut clock);
        assert!(!out.all_dead);
        assert_eq!(out.remaining, ["LeagueofLegends.exe"]);
        assert_eq!(out.rekills, 2);
        assert_eq!(clock.now, Duration::from_secs(6));
    }

    #[test]
    fn readopt_detection() {
        let mut probe = FakeProbe {
            procs: vec![(proc(50, None, "LeagueClient.exe", 1), 1)],
            kills: vec![],
        };
        assert!(is_any_alive(&mut probe, &READOPT_PROCESS_NAMES));
        probe.procs.clear();
        assert!(!is_any_alive(&mut probe, &READOPT_PROCESS_NAMES));
    }
}

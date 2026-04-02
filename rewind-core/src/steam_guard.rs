use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub fn is_steam_running() -> bool {
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_lowercase();
        // "steam" covers Linux and macOS; "steam.exe" covers Windows
        name == "steam" || name == "steam.exe"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_guard_does_not_panic() {
        let _ = is_steam_running();
    }
}

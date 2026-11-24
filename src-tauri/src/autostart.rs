use crate::issue::Issue;
use std::path::Path;

/// The per-user Run key Windows reads at login.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Value name; this is also what Task Manager → Startup shows.
const VALUE: &str = "SAS Launcher";

/// Argument the login entry carries so the launcher can come back hidden.
pub const MINIMIZED_ARG: &str = "--minimized";

/// The command line stored in the Run key. The executable is quoted because the
/// installer path ("…\Programs\SAS Launcher\…") contains a space.
pub fn command(exe: &Path, minimized: bool) -> String {
    let exe = exe.display();
    if minimized {
        format!("\"{exe}\" {MINIMIZED_ARG}")
    } else {
        format!("\"{exe}\"")
    }
}

/// Whether the login entry exists. Any read failure counts as "not set", which
/// is also what the UI should show when the key is unreadable.
pub fn enabled() -> bool {
    windows_registry::CURRENT_USER
        .open(RUN_KEY)
        .and_then(|key| key.get_string(VALUE))
        .is_ok()
}

/// Create, rewrite or delete the login entry. Rewriting is idempotent, so the
/// minimised flag can be flipped without a disable/enable round trip.
pub fn set(exe: &Path, enabled: bool, minimized: bool) -> Result<(), Issue> {
    if !enabled {
        return clear();
    }
    windows_registry::CURRENT_USER
        .create(RUN_KEY)
        .and_then(|key| key.set_string(VALUE, command(exe, minimized)))
        .map_err(|error| Issue::with_detail("autostartWrite", error))
}

fn clear() -> Result<(), Issue> {
    // Two opens on purpose: `write()` alone cannot read the value back, and the
    // read-only handle below is what tells "nothing to delete" from a failure.
    if !enabled() {
        return Ok(());
    }
    windows_registry::CURRENT_USER
        .options()
        .write()
        .open(RUN_KEY)
        .and_then(|key| key.remove_value(VALUE))
        .map_err(|error| Issue::with_detail("autostartWrite", error))
}

/// Whether this launch came from the login entry rather than the user.
pub fn requested_minimized(args: &[String]) -> bool {
    args.iter().any(|arg| arg == MINIMIZED_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_quotes_the_executable_and_only_adds_the_flag_when_hidden() {
        let exe = Path::new(r"C:\Users\me\AppData\Local\Programs\SAS Launcher\sas-launcher.exe");
        assert_eq!(
            command(exe, false),
            r#""C:\Users\me\AppData\Local\Programs\SAS Launcher\sas-launcher.exe""#
        );
        assert_eq!(
            command(exe, true),
            r#""C:\Users\me\AppData\Local\Programs\SAS Launcher\sas-launcher.exe" --minimized"#
        );
    }

    #[test]
    fn the_hidden_flag_is_read_from_the_launch_arguments() {
        let args = ["sas-launcher.exe", "--minimized"].map(str::to_string);
        assert!(requested_minimized(&args));
        let plain = ["sas-launcher.exe"].map(str::to_string);
        assert!(!requested_minimized(&plain));
        let partial = ["sas-launcher.exe", "--minimised"].map(str::to_string);
        assert!(!requested_minimized(&partial));
    }
}

use crate::issue::Issue;
use std::path::Path;

/// The per-user Run key Windows reads at login.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Where Task Manager keeps its own enable/disable choice for Run entries.
/// Disabling does not delete the Run value, it writes a flag here.
const APPROVAL_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

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

/// A `StartupApproved` entry is a little-endian flag followed by a timestamp.
/// The low bit of the flag is set when Task Manager disabled the entry; a
/// missing or trimmed value means Windows treats the entry as enabled.
pub fn approval_disabled(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] & 1 == 1
}

/// Whether the login entry exists *and* Task Manager has not disabled it.
/// Any read failure counts as "not set", which is what the UI should show when
/// the key is unreadable.
pub fn enabled() -> bool {
    let registered = windows_registry::CURRENT_USER
        .open(RUN_KEY)
        .and_then(|key| key.get_string(VALUE))
        .is_ok();
    if !registered {
        return false;
    }
    !approval().is_some_and(|bytes| approval_disabled(&bytes))
}

fn approval() -> Option<Vec<u8>> {
    windows_registry::CURRENT_USER
        .open(APPROVAL_KEY)
        .and_then(|key| key.get_value(VALUE))
        .ok()
        .map(|value| value.as_ref().to_vec())
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
        .map_err(|error| Issue::with_detail("autostartWrite", error))?;
    // Task Manager's "disabled" flag outlives the Run value, so re-enabling has
    // to drop it or the entry still would not start.
    delete_value(APPROVAL_KEY)
}

fn clear() -> Result<(), Issue> {
    delete_value(RUN_KEY)?;
    delete_value(APPROVAL_KEY)
}

/// Delete one of our values. Two opens on purpose: a write handle cannot read
/// the value back, and the read-only handle is what separates "nothing to
/// delete" from a real failure.
fn delete_value(key_path: &str) -> Result<(), Issue> {
    let present = windows_registry::CURRENT_USER
        .open(key_path)
        .and_then(|key| key.get_value(VALUE))
        .is_ok();
    if !present {
        return Ok(());
    }
    windows_registry::CURRENT_USER
        .options()
        .write()
        .open(key_path)
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
    fn the_task_manager_flag_is_read_from_the_low_bit() {
        // 02 = enabled, 03 = disabled, followed by the FILETIME Task Manager writes.
        assert!(!approval_disabled(&[
            2, 0, 0, 0, 0x9f, 0xd1, 0xf9, 5, 0x17, 0x23, 0xdc, 1
        ]));
        assert!(approval_disabled(&[
            3, 0, 0, 0, 0x9f, 0xd1, 0xf9, 5, 0x17, 0x23, 0xdc, 1
        ]));
        assert!(!approval_disabled(&[6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert!(approval_disabled(&[7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        // No entry, or a value Windows would not be able to interpret: enabled.
        assert!(!approval_disabled(&[]));
        assert!(!approval_disabled(&[3]));
        assert!(!approval_disabled(&[3, 0, 0]));
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

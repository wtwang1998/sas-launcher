use crate::issue::Issue;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::Path,
    process::{Child, Command},
};
use windows_sys::Win32::{
    Foundation::{ERROR_INVALID_PARAMETER, FILETIME, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE, WaitForSingleObject,
    },
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub created: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    Exited(Option<u32>),
    Unknown,
}

/// The session-scoped file that runs just before the project's startup file.
pub const STARTUP_PRELUDE: &str = "startup-prelude.sas";

/// SAS resets `SAS_EXECFILEPATH` while a windowed session starts, so a value
/// passed on the command line never reaches the autoexec files (measured: the
/// variable is empty again inside the session). An `OPTIONS SET=` issued inside
/// the session does survive. The prelude stays pure ASCII and reads the path
/// from an environment variable, because the file is read in the session's
/// encoding while the path itself may hold characters that encoding mangles.
pub fn startup_prelude() -> &'static str {
    "options set=SAS_EXECFILEPATH \"%sysget(SASLAUNCHER_STARTUP_FILE)\";\r\n"
}

/// The launch command line, in the order documented in the redesign spec.
/// `-ENCODING`/`-LOCALE` are intentionally absent: they live in the cfg the
/// `-CONFIG` argument points at. A project's own autoexec file is appended with
/// `-APPEND AUTOEXEC`: on Windows that adds it to the autoexec chain the factory
/// cfg already declares instead of replacing the site's file. Projects whose
/// script locates itself also get the prelude and its environment variable.
pub fn arguments(
    cfg: &Path,
    sasuser: &Path,
    title: &str,
    altlog_dir: &Path,
    startup: Option<&Path>,
    prelude: Option<&Path>,
) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = vec![
        "-CONFIG".into(),
        cfg.as_os_str().into(),
        "-SASUSER".into(),
        sasuser.as_os_str().into(),
        "-NORSASUSER".into(),
        "-AWSTITLE".into(),
        title.into(),
        "-ALTLOG".into(),
        altlog_dir.as_os_str().into(),
        "-LOGPARM".into(),
        "WRITE=IMMEDIATE".into(),
    ];
    if let Some(startup) = startup {
        if let Some(prelude) = prelude {
            arguments.push("-SET".into());
            arguments.push("SASLAUNCHER_STARTUP_FILE".into());
            arguments.push(startup.as_os_str().into());
            arguments.push("-APPEND".into());
            arguments.push("AUTOEXEC".into());
            arguments.push(prelude.as_os_str().into());
        }
        arguments.push("-APPEND".into());
        arguments.push("AUTOEXEC".into());
        arguments.push(startup.as_os_str().into());
    }
    arguments
}

pub fn start(exe: &Path, args: &[OsString], cwd: &Path) -> Result<Child, Issue> {
    Command::new(exe)
        .args(args)
        .current_dir(cwd)
        .spawn()
        .map_err(|error| Issue::with_detail("spawnFailed", error))
}

fn creation_time(handle: HANDLE) -> io::Result<u64> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

impl ProcessIdentity {
    pub fn from_child(child: &Child) -> io::Result<Self> {
        Ok(Self {
            pid: child.id(),
            created: creation_time(child.as_raw_handle())?,
        })
    }

    pub fn status(self) -> Status {
        if self.pid == 0 {
            return Status::Unknown;
        }
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                self.pid,
            )
        };
        if handle.is_null() {
            return match io::Error::last_os_error().raw_os_error() {
                Some(code) if code as u32 == ERROR_INVALID_PARAMETER => Status::Exited(None),
                _ => Status::Unknown,
            };
        }
        // Keep the process object alive while comparing its creation time and checking exit.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        match creation_time(handle.as_raw_handle()) {
            Ok(created) if created != self.created => return Status::Exited(None),
            Ok(_) => {}
            Err(_) => return Status::Unknown,
        }
        match unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Status::Running,
            WAIT_OBJECT_0 => {
                let mut code = 0;
                let known = unsafe { GetExitCodeProcess(handle.as_raw_handle(), &mut code) } != 0;
                Status::Exited(known.then_some(code))
            }
            _ => Status::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;

    #[test]
    fn arguments_route_a_log_copy_without_changing_the_log_window() {
        let arguments = arguments(
            Path::new(r"C:\用户设置\配置 一\sasv9.cfg"),
            Path::new(r"C:\用户设置\会话\sasuser"),
            "项目一 · UTF-8",
            Path::new(r"C:\用户设置\会话"),
            None,
            None,
        );
        assert_eq!(
            arguments,
            [
                "-CONFIG",
                r"C:\用户设置\配置 一\sasv9.cfg",
                "-SASUSER",
                r"C:\用户设置\会话\sasuser",
                "-NORSASUSER",
                "-AWSTITLE",
                "项目一 · UTF-8",
                "-ALTLOG",
                r"C:\用户设置\会话",
                "-LOGPARM",
                "WRITE=IMMEDIATE",
            ]
            .map(OsString::from)
        );
        assert!(!arguments.iter().any(|argument| argument == "-ENCODING"));
        assert!(!arguments.iter().any(|argument| argument == "-LOCALE"));
        // -LOG is batch-only; the copy must come from -ALTLOG.
        assert!(!arguments.iter().any(|argument| argument == "-LOG"));
        // A plain -AUTOEXEC would replace the site's file instead of extending it.
        assert!(!arguments.iter().any(|argument| argument == "-AUTOEXEC"));
    }

    #[test]
    fn arguments_append_the_projects_startup_file_after_the_log_copy() {
        let cfg = Path::new("C:/用户设置/配置 一/sasv9.cfg");
        let sasuser = Path::new("C:/用户设置/会话/sasuser");
        let altlog = Path::new("C:/用户设置/会话");
        let startup = Path::new("C:/程序/setup.sas");

        let plain = arguments(cfg, sasuser, "项目一", altlog, None, None);
        let appended = arguments(cfg, sasuser, "项目一", altlog, Some(startup), None);

        let tail: Vec<String> = appended[plain.len()..]
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            tail,
            [
                "-APPEND".to_owned(),
                "AUTOEXEC".to_owned(),
                "C:/程序/setup.sas".to_owned(),
            ]
        );
        // Appending must not touch the copy of the log or the window title.
        assert_eq!(&appended[..plain.len()], plain);
    }

    #[test]
    fn arguments_bridge_the_startup_path_through_the_session_prelude() {
        let cfg = Path::new("C:/配置/sasv9.cfg");
        let sasuser = Path::new("C:/会话/sasuser");
        let altlog = Path::new("C:/会话");
        let startup = Path::new("C:/程序/setup.sas");
        let prelude = Path::new("C:/会话/startup-prelude.sas");

        let hidden = arguments(cfg, sasuser, "项目一", altlog, Some(startup), None);
        let bridged = arguments(cfg, sasuser, "项目一", altlog, Some(startup), Some(prelude));

        let tail = |arguments: &[OsString], from: usize| -> Vec<String> {
            arguments[from..]
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect()
        };
        assert_eq!(
            tail(&hidden, hidden.len() - 3),
            ["-APPEND", "AUTOEXEC", "C:/程序/setup.sas"].map(str::to_owned)
        );
        // The environment variable SAS keeps, the prelude that copies it into
        // SAS_EXECFILEPATH, then the project's own file.
        assert_eq!(
            tail(&bridged, hidden.len() - 3),
            [
                "-SET",
                "SASLAUNCHER_STARTUP_FILE",
                "C:/程序/setup.sas",
                "-APPEND",
                "AUTOEXEC",
                "C:/会话/startup-prelude.sas",
                "-APPEND",
                "AUTOEXEC",
                "C:/程序/setup.sas",
            ]
            .map(str::to_owned)
        );
        // A command-line SAS_EXECFILEPATH never survives a windowed start.
        assert!(
            !bridged
                .iter()
                .any(|argument| argument == "SAS_EXECFILEPATH")
        );
    }

    #[test]
    fn the_prelude_is_ascii_and_reads_the_path_from_the_environment() {
        let text = startup_prelude();
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("options set=SAS_EXECFILEPATH"), "{text}");
        assert!(text.contains("%sysget(SASLAUNCHER_STARTUP_FILE)"), "{text}");
    }

    #[test]
    fn compares_process_creation_time_to_protect_against_pid_reuse() {
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::from_child(&child).unwrap();
        let running = identity.status();
        let wrong_time = ProcessIdentity {
            created: identity.created + 1,
            ..identity
        }
        .status();
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(running, Status::Running);
        assert_eq!(wrong_time, Status::Exited(None));
        assert!(matches!(identity.status(), Status::Exited(_)));
    }
}

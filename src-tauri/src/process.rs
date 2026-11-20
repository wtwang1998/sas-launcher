use crate::discovery::LaunchConfig;
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

pub fn arguments(
    config: &LaunchConfig,
    profile: &Path,
    title: &str,
    encoding: &str,
) -> Vec<OsString> {
    let mut arguments = vec![
        "-CONFIG".into(),
        config.cfg_path().as_os_str().into(),
        "-SASUSER".into(),
        profile.as_os_str().into(),
        "-AWSTITLE".into(),
        title.into(),
        "-NORSASUSER".into(),
        "-ENCODING".into(),
        encoding.to_ascii_lowercase().into(),
    ];
    if let Some(locale) = &config.locale {
        arguments.push("-LOCALE".into());
        arguments.push(locale.into());
    }
    arguments
}

pub fn start(config: &LaunchConfig, args: &[OsString], cwd: &Path) -> Result<Child, Issue> {
    Command::new(&config.key.exe)
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

    #[test]
    fn pass_paths_title_encoding_and_locale_as_distinct_os_arguments() {
        let config = LaunchConfig {
            reference: ConfigRef::BuiltIn {
                exe: r"C:\Program Files\SAS\sas.exe".into(),
                code: "u8".into(),
            },
            key: crate::discovery::ConfigKey {
                exe: r"C:\Program Files\SAS\sas.exe".into(),
                cfg: r"C:\Program Files\SAS\中文\sasv9.cfg".into(),
            },
            code: "u8".into(),
            custom_id: None,
            custom_name: None,
            base: None,
            file: None,
            encoding: Some("UTF-8".into()),
            locale: Some("en_US".into()),
            default_profile: None,
            ui_language: None,
            is_default: false,
            error: None,
        };
        assert_eq!(
            arguments(
                &config,
                Path::new(r"C:\用户设置\会话 一"),
                "项目一 · UTF-8",
                "UTF-8"
            ),
            [
                "-CONFIG",
                r"C:\Program Files\SAS\中文\sasv9.cfg",
                "-SASUSER",
                r"C:\用户设置\会话 一",
                "-AWSTITLE",
                "项目一 · UTF-8",
                "-NORSASUSER",
                "-ENCODING",
                "utf-8",
                "-LOCALE",
                "en_US",
            ]
            .map(OsString::from)
        );
    }

    use crate::discovery::ConfigRef;
    use std::os::windows::process::CommandExt;
}

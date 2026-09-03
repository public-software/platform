//! The base directories: the XDG Base Directory Specification on every platform, with the
//! platform's own defaults.

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::{Env, Platform};

/// Why a set of directories could not be resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// No home directory to hang a default off: `HOME` (`USERPROFILE` on Windows) is not set
    /// to an absolute path and a variable that would replace the default is not set either.
    NoHome {
        /// The platform whose table needed the home directory.
        platform: Platform,
    },
    /// The text is not a component name: lowercase ASCII words joined by single hyphens.
    InvalidAppId(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHome { platform: Platform::Windows } => f.write_str(
                "no home directory: USERPROFILE (or APPDATA and LOCALAPPDATA) is not set to an absolute path",
            ),
            Self::NoHome { .. } => {
                f.write_str("no home directory: HOME is not set to an absolute path")
            }
            Self::InvalidAppId(text) => write!(
                f,
                "{text:?} is not a component name (lowercase words joined by hyphens, like docs-aggregate)"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// The base directories of one user on one platform.
///
/// The five directories of the specification (`config_home`, `data_home`, `state_home`,
/// `cache_home`, `runtime_dir`) and its two search lists (`config_dirs`, `data_dirs`),
/// resolved by [`BaseDirs::resolve`]. Programs place their own files under them through
/// [`BaseDirs::for_app`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseDirs {
    /// The platform whose table was applied.
    pub platform: Platform,
    /// Where user-specific configuration files go (`$XDG_CONFIG_HOME`).
    pub config_home: PathBuf,
    /// Where user-specific data files go (`$XDG_DATA_HOME`).
    pub data_home: PathBuf,
    /// Where user-specific state files go: logs, history, recently used files
    /// (`$XDG_STATE_HOME`).
    pub state_home: PathBuf,
    /// Where user-specific non-essential, regenerable files go (`$XDG_CACHE_HOME`).
    pub cache_home: PathBuf,
    /// Where user-specific runtime files and sockets go (`$XDG_RUNTIME_DIR`).
    ///
    /// The specification gives it no default and tells an application to "fall back to a
    /// replacement directory with similar capabilities and print a warning message" when it is
    /// unset; that fallback is the caller's, so this is `None` when the platform offers no
    /// directory with the specification's guarantees (owned by the user, mode `0700`, bound to
    /// the login session).
    pub runtime_dir: Option<PathBuf>,
    /// The system-wide configuration directories to search after `config_home`, most
    /// important first (`$XDG_CONFIG_DIRS`).
    pub config_dirs: Vec<PathBuf>,
    /// The system-wide data directories to search after `data_home`, most important first
    /// (`$XDG_DATA_DIRS`).
    pub data_dirs: Vec<PathBuf>,
}

impl BaseDirs {
    /// Resolves the base directories of `platform` from `env`.
    ///
    /// One rule on every platform: a variable of the specification that is set to an absolute
    /// path wins (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`,
    /// `XDG_RUNTIME_DIR`, and the lists `XDG_CONFIG_DIRS`, `XDG_DATA_DIRS`). A variable that is
    /// unset or empty takes the platform's default; one set to a relative path is invalid and
    /// ignored, as the specification says, so it takes the default too. An entry of a list that
    /// is empty or relative is dropped and the rest keep their order; a list that is set but has
    /// no valid entry is empty, not the default.
    ///
    /// The defaults:
    ///
    /// | | [`Xdg`](Platform::Xdg) | [`MacOs`](Platform::MacOs) | [`Windows`](Platform::Windows) |
    /// |---|---|---|---|
    /// | `config_home` | `$HOME/.config` | `~/Library/Application Support` | `%APPDATA%` |
    /// | `data_home` | `$HOME/.local/share` | `~/Library/Application Support` | `%APPDATA%` |
    /// | `state_home` | `$HOME/.local/state` | `~/Library/Application Support` | `%LOCALAPPDATA%` |
    /// | `cache_home` | `$HOME/.cache` | `~/Library/Caches` | `%LOCALAPPDATA%` |
    /// | `runtime_dir` | none | `$TMPDIR` when absolute | none |
    /// | `config_dirs` | `/etc/xdg` | `/Library/Application Support` | `%ProgramData%` |
    /// | `data_dirs` | `/usr/local/share/`, `/usr/share/` | `/Library/Application Support` | `%ProgramData%` |
    ///
    /// On Windows, `%APPDATA%` and `%LOCALAPPDATA%` fall back to `%USERPROFILE%\AppData\Roaming`
    /// and `...\Local`, and `%ProgramData%` to `%ALLUSERSPROFILE%`, then `C:\ProgramData`, the
    /// documented defaults of the known folders. macOS keeps configuration under
    /// `Application Support` because Apple reserves `Library/Preferences` for the defaults
    /// system ("you should never create files in this directory yourself").
    ///
    /// # Errors
    ///
    /// [`Error::NoHome`] when a default is needed and the home directory (`HOME`, or
    /// `USERPROFILE` on Windows) is not set to an absolute path. A fully specified environment
    /// needs no home and resolves without one.
    pub fn resolve(platform: Platform, env: &Env) -> Result<Self, Error> {
        let table = Table { platform, env };
        Ok(Self {
            platform,
            config_home: table.home_dir("XDG_CONFIG_HOME", Table::config_default)?,
            data_home: table.home_dir("XDG_DATA_HOME", Table::data_default)?,
            state_home: table.home_dir("XDG_STATE_HOME", Table::state_default)?,
            cache_home: table.home_dir("XDG_CACHE_HOME", Table::cache_default)?,
            runtime_dir: table
                .absolute("XDG_RUNTIME_DIR")
                .or_else(|| table.runtime_default()),
            config_dirs: table
                .list("XDG_CONFIG_DIRS")
                .unwrap_or_else(|| table.config_dirs_default()),
            data_dirs: table
                .list("XDG_DATA_DIRS")
                .unwrap_or_else(|| table.data_dirs_default()),
        })
    }
}

/// One platform's default table over one environment.
struct Table<'a> {
    platform: Platform,
    env: &'a Env,
}

impl Table<'_> {
    /// The value of `name` when it is set to an absolute path.
    fn absolute(&self, name: &str) -> Option<PathBuf> {
        self.env
            .lookup(self.platform, name)
            .filter(|v| self.platform.is_absolute(v))
            .map(PathBuf::from)
    }

    /// A directory of the specification: the variable when absolute, else the default.
    fn home_dir(
        &self,
        name: &str,
        default: fn(&Self) -> Result<PathBuf, Error>,
    ) -> Result<PathBuf, Error> {
        match self.absolute(name) {
            Some(dir) => Ok(dir),
            None => default(self),
        }
    }

    /// A search list of the specification: `None` when the variable is unset or empty (the
    /// default applies), else its absolute entries in order.
    fn list(&self, name: &str) -> Option<Vec<PathBuf>> {
        let value = self
            .env
            .lookup(self.platform, name)
            .filter(|v| !v.is_empty())?;
        Some(split_list(self.platform, value))
    }

    /// The user's home: `HOME`, or `USERPROFILE` on Windows, when absolute.
    fn home(&self) -> Result<PathBuf, Error> {
        let name = match self.platform {
            Platform::Xdg | Platform::MacOs => "HOME",
            Platform::Windows => "USERPROFILE",
        };
        self.absolute(name).ok_or(Error::NoHome {
            platform: self.platform,
        })
    }

    fn under_home(&self, segments: &[&str]) -> Result<PathBuf, Error> {
        Ok(self.platform.join(&self.home()?, segments))
    }

    /// Windows: `%APPDATA%`, else `%USERPROFILE%\AppData\Roaming`.
    fn roaming(&self) -> Result<PathBuf, Error> {
        match self.absolute("APPDATA") {
            Some(dir) => Ok(dir),
            None => self.under_home(&["AppData", "Roaming"]),
        }
    }

    /// Windows: `%LOCALAPPDATA%`, else `%USERPROFILE%\AppData\Local`.
    fn local(&self) -> Result<PathBuf, Error> {
        match self.absolute("LOCALAPPDATA") {
            Some(dir) => Ok(dir),
            None => self.under_home(&["AppData", "Local"]),
        }
    }

    /// Windows: `%ProgramData%`, else `%ALLUSERSPROFILE%`, else the documented default.
    fn program_data(&self) -> PathBuf {
        self.absolute("ProgramData")
            .or_else(|| self.absolute("ALLUSERSPROFILE"))
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
    }

    fn config_default(&self) -> Result<PathBuf, Error> {
        match self.platform {
            Platform::Xdg => self.under_home(&[".config"]),
            Platform::MacOs => self.under_home(&["Library", "Application Support"]),
            Platform::Windows => self.roaming(),
        }
    }

    fn data_default(&self) -> Result<PathBuf, Error> {
        match self.platform {
            Platform::Xdg => self.under_home(&[".local", "share"]),
            Platform::MacOs => self.under_home(&["Library", "Application Support"]),
            Platform::Windows => self.roaming(),
        }
    }

    fn state_default(&self) -> Result<PathBuf, Error> {
        match self.platform {
            Platform::Xdg => self.under_home(&[".local", "state"]),
            Platform::MacOs => self.under_home(&["Library", "Application Support"]),
            Platform::Windows => self.local(),
        }
    }

    fn cache_default(&self) -> Result<PathBuf, Error> {
        match self.platform {
            Platform::Xdg => self.under_home(&[".cache"]),
            Platform::MacOs => self.under_home(&["Library", "Caches"]),
            Platform::Windows => self.local(),
        }
    }

    fn runtime_default(&self) -> Option<PathBuf> {
        match self.platform {
            // The specification gives no default, and nothing on Windows carries its guarantees.
            Platform::Xdg | Platform::Windows => None,
            // Per user, created by the system with mode 0700, cleaned when the session ends.
            Platform::MacOs => self.absolute("TMPDIR"),
        }
    }

    fn config_dirs_default(&self) -> Vec<PathBuf> {
        match self.platform {
            Platform::Xdg => vec![PathBuf::from("/etc/xdg")],
            Platform::MacOs => vec![PathBuf::from("/Library/Application Support")],
            Platform::Windows => vec![self.program_data()],
        }
    }

    fn data_dirs_default(&self) -> Vec<PathBuf> {
        match self.platform {
            Platform::Xdg => vec![
                PathBuf::from("/usr/local/share/"),
                PathBuf::from("/usr/share/"),
            ],
            Platform::MacOs => vec![PathBuf::from("/Library/Application Support")],
            Platform::Windows => vec![self.program_data()],
        }
    }
}

/// The absolute entries of a search list, in order; empty and relative entries are dropped.
///
/// A list is split as text: an entry that is not valid Unicode is kept with the replacement
/// character where its bytes were not, which the specification's absolute-path rule then judges
/// like any other entry.
fn split_list(platform: Platform, value: &OsStr) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    for entry in value.to_string_lossy().split(platform.list_separator()) {
        push_entry(platform, entry, &mut entries);
    }
    entries
}

fn push_entry(platform: Platform, entry: &str, entries: &mut Vec<PathBuf>) {
    let entry = Path::new(entry);
    if platform.is_absolute(entry.as_os_str()) {
        entries.push(entry.to_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Env {
        pairs.iter().copied().collect()
    }

    #[test]
    fn a_list_splits_on_the_platform_separator() {
        let unix = split_list(Platform::Xdg, OsStr::new("/a:/b/::c:/d"));
        assert_eq!(
            unix,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b/"),
                PathBuf::from("/d")
            ]
        );
        let windows = split_list(Platform::Windows, OsStr::new(r"C:\a;D:\b;relative;;"));
        assert_eq!(
            windows,
            vec![PathBuf::from(r"C:\a"), PathBuf::from(r"D:\b")]
        );
        // A colon inside a Windows drive letter is not a list separator there.
        assert_eq!(
            split_list(Platform::Windows, OsStr::new(r"C:\a")),
            vec![PathBuf::from(r"C:\a")]
        );
        assert_eq!(
            split_list(Platform::Xdg, OsStr::new("")),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn errors_name_the_variable_that_was_missing() {
        assert!(
            Error::NoHome {
                platform: Platform::Xdg
            }
            .to_string()
            .contains("HOME")
        );
        assert!(
            Error::NoHome {
                platform: Platform::MacOs
            }
            .to_string()
            .contains("HOME")
        );
        let windows = Error::NoHome {
            platform: Platform::Windows,
        }
        .to_string();
        assert!(
            windows.contains("USERPROFILE") && windows.contains("APPDATA"),
            "{windows}"
        );
        let app = Error::InvalidAppId("Bad".into()).to_string();
        assert!(
            app.contains("\"Bad\"") && app.contains("component"),
            "{app}"
        );
    }

    #[test]
    fn windows_program_data_falls_back_through_allusersprofile() {
        let dirs = BaseDirs::resolve(
            Platform::Windows,
            &env(&[
                ("USERPROFILE", r"C:\Users\u"),
                ("ALLUSERSPROFILE", r"D:\AllUsers"),
            ]),
        )
        .unwrap();
        assert_eq!(dirs.config_dirs, vec![PathBuf::from(r"D:\AllUsers")]);
        assert_eq!(dirs.data_dirs, vec![PathBuf::from(r"D:\AllUsers")]);
    }

    #[test]
    fn windows_roaming_and_local_are_independent_of_each_other() {
        let dirs = BaseDirs::resolve(
            Platform::Windows,
            &env(&[
                ("USERPROFILE", r"C:\Users\u"),
                ("LOCALAPPDATA", r"E:\Local"),
            ]),
        )
        .unwrap();
        assert_eq!(
            dirs.config_home,
            PathBuf::from(r"C:\Users\u\AppData\Roaming")
        );
        assert_eq!(dirs.data_home, PathBuf::from(r"C:\Users\u\AppData\Roaming"));
        assert_eq!(dirs.state_home, PathBuf::from(r"E:\Local"));
        assert_eq!(dirs.cache_home, PathBuf::from(r"E:\Local"));
    }

    #[test]
    fn windows_needs_no_profile_when_both_app_data_folders_are_set() {
        let dirs = BaseDirs::resolve(
            Platform::Windows,
            &env(&[("APPDATA", r"C:\R"), ("LOCALAPPDATA", r"C:\L")]),
        )
        .unwrap();
        assert_eq!(dirs.config_home, PathBuf::from(r"C:\R"));
        assert_eq!(dirs.cache_home, PathBuf::from(r"C:\L"));
    }

    #[test]
    fn windows_lists_split_on_semicolons() {
        let dirs = BaseDirs::resolve(
            Platform::Windows,
            &env(&[
                ("USERPROFILE", r"C:\Users\u"),
                ("XDG_DATA_DIRS", r"C:\a;D:\b"),
            ]),
        )
        .unwrap();
        assert_eq!(
            dirs.data_dirs,
            vec![PathBuf::from(r"C:\a"), PathBuf::from(r"D:\b")]
        );
    }
}

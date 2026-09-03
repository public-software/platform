//! The target platform: which default table applies and how paths are spelled.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

/// The family of conventions a set of directories follows.
///
/// Resolution is a function of the platform and an [`Env`](crate::Env), never of the host the
/// code runs on, so every table can be exercised on every host. [`Platform::current`] names the
/// host's own family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Linux, the BSDs and everything else that follows the XDG Base Directory Specification.
    Xdg,
    /// macOS: the `Library` layout of Apple's File System Programming Guide.
    MacOs,
    /// Windows: the known folders (`FOLDERID_RoamingAppData`, `FOLDERID_LocalAppData`,
    /// `FOLDERID_ProgramData`) as their environment variables expose them.
    Windows,
}

impl Platform {
    /// The family the running program's host belongs to.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Xdg
        }
    }

    /// The path separator this platform spells directories with.
    #[must_use]
    pub const fn separator(self) -> char {
        match self {
            Self::Xdg | Self::MacOs => '/',
            Self::Windows => '\\',
        }
    }

    /// The separator between the entries of a search list (`XDG_DATA_DIRS`, `XDG_CONFIG_DIRS`).
    ///
    /// The specification writes the lists with `:`; on Windows a `:` is part of every drive
    /// letter, so the `PATH` convention there, `;`, applies instead.
    #[must_use]
    pub const fn list_separator(self) -> char {
        match self {
            Self::Xdg | Self::MacOs => ':',
            Self::Windows => ';',
        }
    }

    /// Whether `value` is an absolute path as this platform reads it.
    ///
    /// The specification requires every variable to hold an absolute path and says an
    /// implementation "should consider the path invalid and ignore it" otherwise. On the
    /// Unix-like platforms that is a leading `/`; on Windows a drive letter followed by `:\` or
    /// `:/`, or a UNC prefix `\\`. The host's own notion of an absolute path is deliberately not
    /// consulted.
    #[must_use]
    pub fn is_absolute(self, value: &OsStr) -> bool {
        let bytes = value.as_encoded_bytes();
        match self {
            Self::Xdg | Self::MacOs => bytes.first() == Some(&b'/'),
            Self::Windows => {
                let drive = bytes.len() >= 3
                    && bytes[0].is_ascii_alphabetic()
                    && bytes[1] == b':'
                    && (bytes[2] == b'\\' || bytes[2] == b'/');
                drive || bytes.starts_with(b"\\\\")
            }
        }
    }

    /// Whether `byte` ends a directory name on this platform.
    const fn is_separator_byte(self, byte: u8) -> bool {
        match self {
            Self::Xdg | Self::MacOs => byte == b'/',
            Self::Windows => byte == b'\\' || byte == b'/',
        }
    }

    /// `base` followed by `segments`, spelled with this platform's separator.
    ///
    /// A trailing separator on `base` is not doubled; the segments are taken verbatim.
    #[must_use]
    pub fn join<S: AsRef<str>>(self, base: &Path, segments: &[S]) -> PathBuf {
        let mut out: OsString = base.as_os_str().to_os_string();
        for segment in segments {
            let ends_with_separator = out
                .as_encoded_bytes()
                .last()
                .is_some_and(|b| self.is_separator_byte(*b));
            if !ends_with_separator && !out.is_empty() {
                out.push(self.separator().to_string());
            }
            out.push(segment.as_ref());
        }
        PathBuf::from(out)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Xdg => "xdg",
            Self::MacOs => "macos",
            Self::Windows => "windows",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_absolute_is_a_leading_slash() {
        for platform in [Platform::Xdg, Platform::MacOs] {
            assert!(platform.is_absolute(OsStr::new("/")));
            assert!(platform.is_absolute(OsStr::new("/home/u")));
            assert!(!platform.is_absolute(OsStr::new("")));
            assert!(!platform.is_absolute(OsStr::new("home")));
            assert!(!platform.is_absolute(OsStr::new("~/x")));
            assert!(!platform.is_absolute(OsStr::new(r"C:\x")));
        }
    }

    #[test]
    fn windows_absolute_is_a_drive_or_unc_prefix() {
        let w = Platform::Windows;
        assert!(w.is_absolute(OsStr::new(r"C:\")));
        assert!(w.is_absolute(OsStr::new("d:/x")));
        assert!(w.is_absolute(OsStr::new(r"\\server\share")));
        assert!(!w.is_absolute(OsStr::new("C:")));
        assert!(!w.is_absolute(OsStr::new("C:x")));
        assert!(!w.is_absolute(OsStr::new("/x")));
        assert!(!w.is_absolute(OsStr::new(r"\x")));
        assert!(!w.is_absolute(OsStr::new("1:\\x")));
        assert!(!w.is_absolute(OsStr::new("")));
    }

    #[test]
    fn join_uses_the_target_separator_and_never_doubles_it() {
        assert_eq!(
            Platform::Xdg.join(Path::new("/home/u"), &[".local", "share"]),
            PathBuf::from("/home/u/.local/share")
        );
        assert_eq!(
            Platform::Xdg.join(Path::new("/home/u/"), &["x"]),
            PathBuf::from("/home/u/x")
        );
        assert_eq!(
            Platform::Xdg.join(Path::new("/"), &["etc"]),
            PathBuf::from("/etc")
        );
        assert_eq!(
            Platform::Windows.join(Path::new(r"C:\Users\u"), &["AppData", "Local"]),
            PathBuf::from(r"C:\Users\u\AppData\Local")
        );
        assert_eq!(
            Platform::Windows.join(Path::new(r"C:\Users\u\"), &["x"]),
            PathBuf::from(r"C:\Users\u\x")
        );
        assert_eq!(
            Platform::Windows.join(Path::new("C:/Users/u/"), &["x"]),
            PathBuf::from("C:/Users/u/x")
        );
        let none: [&str; 0] = [];
        assert_eq!(
            Platform::MacOs.join(Path::new("/x/"), &none),
            PathBuf::from("/x/")
        );
    }

    #[test]
    fn separators_and_names_per_platform() {
        assert_eq!(Platform::Xdg.separator(), '/');
        assert_eq!(Platform::MacOs.separator(), '/');
        assert_eq!(Platform::Windows.separator(), '\\');
        assert_eq!(Platform::Xdg.list_separator(), ':');
        assert_eq!(Platform::Windows.list_separator(), ';');
        assert_eq!(Platform::Xdg.to_string(), "xdg");
        assert_eq!(Platform::MacOs.to_string(), "macos");
        assert_eq!(Platform::Windows.to_string(), "windows");
    }
}

//! `pub-platform-paths` — the `paths` library of [`platform`](https://github.com/public-software/platform).
//!
//! Where a program of the suite keeps its files: the base directories of the XDG Base Directory
//! Specification (configuration, data, state, cache, runtime, and the two system-wide search
//! lists) resolved for Linux, macOS and Windows from an explicit environment map, and the
//! suite's own directory for each component under them. The specification is listed in the
//! repository's `PROVENANCE.md`, with Apple's and Microsoft's documentation of the native
//! folders the macOS and Windows tables default to.
//!
//! Resolution is a pure function of a [`Platform`] and an [`Env`]: nothing here reads the
//! process environment unless a caller asks for [`Env::from_process`], nothing touches the
//! disk, and every platform's table can be exercised on every host.
//!
//! ```
//! use std::path::PathBuf;
//! use pub_platform_paths::{AppId, BaseDirs, Env, Platform};
//!
//! # fn main() -> Result<(), pub_platform_paths::Error> {
//! let env: Env = [("HOME", "/home/u"), ("XDG_CONFIG_HOME", "/etc/u")].into_iter().collect();
//! let dirs = BaseDirs::resolve(Platform::Xdg, &env)?;
//! assert_eq!(dirs.config_home, PathBuf::from("/etc/u"));
//! assert_eq!(dirs.data_home, PathBuf::from("/home/u/.local/share"));
//!
//! let app = dirs.for_app(&AppId::new("paths")?);
//! assert_eq!(app.config_dir, PathBuf::from("/etc/u/public-software/paths"));
//! assert_eq!(app.cache_dir, PathBuf::from("/home/u/.cache/public-software/paths"));
//! # Ok(())
//! # }
//! ```
//!
//! A program at its edge does `BaseDirs::resolve(Platform::current(), &Env::from_process())`.
//!
//! ```
//! assert_eq!(pub_platform_paths::NAME, "pub-platform-paths");
//! ```

#![forbid(unsafe_code)]

mod app;
mod base;
mod env;
mod platform;

pub use app::{AppDirs, AppId, BUNDLE_PREFIX, SUITE_DIR, VENDOR_DIR};
pub use base::{BaseDirs, Error};
pub use env::Env;
pub use platform::Platform;

/// The crate's name, as `CATALOG.toml` and crates.io know it.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The crate's version, as Cargo knows it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_follows_the_naming_rule() {
        assert_eq!(NAME, "pub-platform-paths");
        assert!(NAME.starts_with("pub-platform-"));
    }

    #[test]
    fn version_is_semver_shaped() {
        assert_eq!(VERSION.split('.').count(), 3, "{VERSION}");
    }
}

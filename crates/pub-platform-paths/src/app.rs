//! The suite's own layout under the base directories: one directory per component.

use std::path::{Path, PathBuf};

use crate::{BaseDirs, Error, Platform};

/// The directory every suite program shares on an XDG platform: `<base>/public-software/`.
pub const SUITE_DIR: &str = "public-software";

/// The prefix of a component's bundle identifier on macOS: `dev.publicsoftware.<component>`,
/// the reverse-DNS name of <https://publicsoftware.dev> as Apple's convention wants it.
pub const BUNDLE_PREFIX: &str = "dev.publicsoftware";

/// The vendor directory on Windows: `<base>\Public Software\<component>`.
pub const VENDOR_DIR: &str = "Public Software";

/// A program's identity for its directories: the component name from `CATALOG.toml`.
///
/// The name is lowercase ASCII words joined by single hyphens, the rule every component of the
/// suite follows (`paths`, `docs-aggregate`), so it is valid as a directory name everywhere.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppId {
    component: String,
}

impl AppId {
    /// An identity for `component`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidAppId`] unless `component` is one or more lowercase ASCII words of letters
    /// and digits, joined by single hyphens.
    pub fn new(component: &str) -> Result<Self, Error> {
        let well_formed = !component.is_empty()
            && component.split('-').all(|word| {
                !word.is_empty()
                    && word
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            });
        if well_formed {
            Ok(Self {
                component: component.to_owned(),
            })
        } else {
            Err(Error::InvalidAppId(component.to_owned()))
        }
    }

    /// The component name.
    #[must_use]
    pub fn component(&self) -> &str {
        &self.component
    }

    /// The path segments this component adds under a base directory of `platform`.
    ///
    /// XDG: `public-software/<component>`; macOS: `dev.publicsoftware.<component>`; Windows:
    /// `Public Software\<component>`.
    #[must_use]
    pub fn segments(&self, platform: Platform) -> Vec<String> {
        match platform {
            Platform::Xdg => vec![SUITE_DIR.to_owned(), self.component.clone()],
            Platform::MacOs => vec![format!("{BUNDLE_PREFIX}.{}", self.component)],
            Platform::Windows => vec![VENDOR_DIR.to_owned(), self.component.clone()],
        }
    }

    /// `base` with this component's segments appended, spelled for `platform`.
    #[must_use]
    pub fn under(&self, platform: Platform, base: &Path) -> PathBuf {
        platform.join(base, &self.segments(platform))
    }
}

/// One component's directories under a user's [`BaseDirs`].
///
/// Every field is the corresponding base with the component's segments appended; nothing is
/// created on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    /// The component these directories belong to.
    pub app: AppId,
    /// The platform whose layout was applied.
    pub platform: Platform,
    /// The component's configuration directory, under `config_home`.
    pub config_dir: PathBuf,
    /// The component's data directory, under `data_home`.
    pub data_dir: PathBuf,
    /// The component's state directory, under `state_home`.
    pub state_dir: PathBuf,
    /// The component's cache directory, under `cache_home`.
    pub cache_dir: PathBuf,
    /// The component's runtime directory, under `runtime_dir` when there is one.
    pub runtime_dir: Option<PathBuf>,
    /// The component's system-wide configuration directories, under each of `config_dirs`.
    pub config_dirs: Vec<PathBuf>,
    /// The component's system-wide data directories, under each of `data_dirs`.
    pub data_dirs: Vec<PathBuf>,
}

impl AppDirs {
    /// The configuration directories to search, most important first: `config_dir`, then
    /// `config_dirs` in order.
    pub fn config_search(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.config_dir.as_path())
            .chain(self.config_dirs.iter().map(PathBuf::as_path))
    }

    /// The data directories to search, most important first: `data_dir`, then `data_dirs` in
    /// order.
    pub fn data_search(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.data_dir.as_path()).chain(self.data_dirs.iter().map(PathBuf::as_path))
    }
}

impl BaseDirs {
    /// The directories of `app` under these bases.
    #[must_use]
    pub fn for_app(&self, app: &AppId) -> AppDirs {
        let platform = self.platform;
        let under = |base: &Path| app.under(platform, base);
        AppDirs {
            app: app.clone(),
            platform,
            config_dir: under(&self.config_home),
            data_dir: under(&self.data_home),
            state_dir: under(&self.state_home),
            cache_dir: under(&self.cache_home),
            runtime_dir: self.runtime_dir.as_deref().map(under),
            config_dirs: self.config_dirs.iter().map(|d| under(d)).collect(),
            data_dirs: self.data_dirs.iter().map(|d| under(d)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_per_platform() {
        let app = AppId::new("docs-aggregate").unwrap();
        assert_eq!(
            app.segments(Platform::Xdg),
            vec!["public-software", "docs-aggregate"]
        );
        assert_eq!(
            app.segments(Platform::MacOs),
            vec!["dev.publicsoftware.docs-aggregate"]
        );
        assert_eq!(
            app.segments(Platform::Windows),
            vec!["Public Software", "docs-aggregate"]
        );
    }

    #[test]
    fn digits_are_allowed_in_a_component_name() {
        assert!(AppId::new("x11").is_ok());
        assert!(AppId::new("3d").is_ok());
        assert!(AppId::new("a-1-b").is_ok());
        assert!(AppId::new("a b").is_err());
        assert!(AppId::new("é").is_err());
    }
}

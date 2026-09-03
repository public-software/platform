//! An explicit environment map: the only input resolution reads.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

use crate::Platform;

/// The environment variables resolution reads, as an explicit map.
///
/// Nothing in this crate reads the process environment on its own: a caller builds an `Env`
/// (from pairs, or with [`Env::from_process`] as the one edge that snapshots the live
/// environment) and passes it in, so resolution is deterministic and a test can describe any
/// machine.
///
/// ```
/// use pub_platform_paths::Env;
///
/// let env: Env = [("HOME", "/home/u"), ("XDG_CONFIG_HOME", "")].into_iter().collect();
/// assert_eq!(env.get("HOME").and_then(|v| v.to_str()), Some("/home/u"));
/// assert_eq!(env.get("XDG_CONFIG_HOME").and_then(|v| v.to_str()), Some(""));
/// assert_eq!(env.get("XDG_DATA_HOME"), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Env {
    vars: BTreeMap<OsString, OsString>,
}

impl Env {
    /// An environment with no variable set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A snapshot of the running process's environment.
    ///
    /// This is the one place the crate touches the ambient environment; call it at the edge of
    /// a program and pass the result down.
    #[must_use]
    pub fn from_process() -> Self {
        std::env::vars_os().collect()
    }

    /// The value of `name`, when it is set (an empty value is set).
    ///
    /// The lookup is exact; see [`Env::lookup`] for the platform's own rule.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&OsStr> {
        self.vars.get(OsStr::new(name)).map(OsString::as_os_str)
    }

    /// The value of `name` as `platform` would find it.
    ///
    /// Windows treats variable names without regard to ASCII case (`ProgramData` and
    /// `PROGRAMDATA` are the same variable), so the Windows table looks them up that way; the
    /// Unix-like platforms match exactly.
    #[must_use]
    pub fn lookup(&self, platform: Platform, name: &str) -> Option<&OsStr> {
        match platform {
            Platform::Xdg | Platform::MacOs => self.get(name),
            Platform::Windows => self.get(name).or_else(|| {
                self.vars
                    .iter()
                    .find(|(k, _)| k.to_str().is_some_and(|k| k.eq_ignore_ascii_case(name)))
                    .map(|(_, v)| v.as_os_str())
            }),
        }
    }

    /// Sets `name` to `value`, replacing any earlier value.
    pub fn set<K: Into<OsString>, V: Into<OsString>>(&mut self, name: K, value: V) -> &mut Self {
        self.vars.insert(name.into(), value.into());
        self
    }

    /// Removes `name`, so it reads as unset.
    pub fn unset(&mut self, name: &str) -> &mut Self {
        self.vars.remove(OsStr::new(name));
        self
    }

    /// The number of variables set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Whether no variable is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    /// The variables, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.as_os_str(), v.as_os_str()))
    }
}

impl<K: Into<OsString>, V: Into<OsString>> FromIterator<(K, V)> for Env {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut env = Self::new();
        env.extend(iter);
        env
    }
}

impl<K: Into<OsString>, V: Into<OsString>> Extend<(K, V)> for Env {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.set(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_unset_and_iterate() {
        let mut env = Env::new();
        assert!(env.is_empty());
        env.set("B", "2").set("A", "1");
        assert_eq!(env.len(), 2);
        let names: Vec<&OsStr> = env.iter().map(|(k, _)| k).collect();
        assert_eq!(names, vec![OsStr::new("A"), OsStr::new("B")]);
        env.set("A", "one");
        assert_eq!(env.get("A"), Some(OsStr::new("one")));
        env.unset("A");
        assert_eq!(env.get("A"), None);
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn windows_lookup_ignores_ascii_case_the_others_do_not() {
        let env: Env = [("ProgramData", r"C:\ProgramData")].into_iter().collect();
        assert_eq!(
            env.lookup(Platform::Windows, "PROGRAMDATA"),
            Some(OsStr::new(r"C:\ProgramData"))
        );
        assert_eq!(
            env.lookup(Platform::Windows, "ProgramData"),
            Some(OsStr::new(r"C:\ProgramData"))
        );
        assert_eq!(env.lookup(Platform::Xdg, "PROGRAMDATA"), None);
        assert_eq!(env.lookup(Platform::MacOs, "PROGRAMDATA"), None);
        assert_eq!(env.lookup(Platform::Windows, "APPDATA"), None);
    }

    #[test]
    fn an_exact_match_wins_over_a_case_insensitive_one_on_windows() {
        let env: Env = [("Path", "mixed"), ("PATH", "upper")].into_iter().collect();
        assert_eq!(
            env.lookup(Platform::Windows, "PATH"),
            Some(OsStr::new("upper"))
        );
    }
}

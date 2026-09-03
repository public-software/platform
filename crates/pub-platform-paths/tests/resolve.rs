//! Path resolution against an explicit environment map, one table per platform.
//!
//! Every test runs on every host: resolution is a function of `(Platform, Env)` and paths are
//! joined with the target platform's separator, never the host's.

use std::path::{Path, PathBuf};

use pub_platform_paths::{AppId, BaseDirs, Env, Error, Platform};

fn env(pairs: &[(&str, &str)]) -> Env {
    pairs.iter().copied().collect()
}

fn xdg(pairs: &[(&str, &str)]) -> BaseDirs {
    BaseDirs::resolve(Platform::Xdg, &env(pairs)).expect("HOME is set")
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn paths(v: &[&str]) -> Vec<PathBuf> {
    v.iter().map(|s| PathBuf::from(*s)).collect()
}

// --- the XDG Base Directory Specification, section "Basics" ---

#[test]
fn xdg_defaults_hang_off_home() {
    let dirs = xdg(&[("HOME", "/home/u")]);
    assert_eq!(dirs.config_home, p("/home/u/.config"));
    assert_eq!(dirs.data_home, p("/home/u/.local/share"));
    assert_eq!(dirs.state_home, p("/home/u/.local/state"));
    assert_eq!(dirs.cache_home, p("/home/u/.cache"));
    assert_eq!(dirs.runtime_dir, None);
    assert_eq!(dirs.config_dirs, paths(&["/etc/xdg"]));
    assert_eq!(dirs.data_dirs, paths(&["/usr/local/share/", "/usr/share/"]));
}

#[test]
fn an_absolute_xdg_variable_beats_the_default() {
    let dirs = xdg(&[
        ("HOME", "/home/u"),
        ("XDG_CONFIG_HOME", "/etc/u-config"),
        ("XDG_DATA_HOME", "/srv/data"),
        ("XDG_STATE_HOME", "/srv/state"),
        ("XDG_CACHE_HOME", "/var/cache/u"),
        ("XDG_RUNTIME_DIR", "/run/user/1000"),
    ]);
    assert_eq!(dirs.config_home, p("/etc/u-config"));
    assert_eq!(dirs.data_home, p("/srv/data"));
    assert_eq!(dirs.state_home, p("/srv/state"));
    assert_eq!(dirs.cache_home, p("/var/cache/u"));
    assert_eq!(dirs.runtime_dir, Some(p("/run/user/1000")));
}

#[test]
fn a_relative_xdg_variable_is_invalid_and_ignored() {
    let dirs = xdg(&[
        ("HOME", "/home/u"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "./share"),
        ("XDG_STATE_HOME", "~/state"),
        ("XDG_CACHE_HOME", "../cache"),
        ("XDG_RUNTIME_DIR", "run"),
    ]);
    assert_eq!(dirs.config_home, p("/home/u/.config"));
    assert_eq!(dirs.data_home, p("/home/u/.local/share"));
    assert_eq!(dirs.state_home, p("/home/u/.local/state"));
    assert_eq!(dirs.cache_home, p("/home/u/.cache"));
    assert_eq!(dirs.runtime_dir, None);
}

#[test]
fn an_empty_xdg_variable_takes_the_default() {
    let dirs = xdg(&[
        ("HOME", "/home/u"),
        ("XDG_CONFIG_HOME", ""),
        ("XDG_DATA_DIRS", ""),
        ("XDG_CONFIG_DIRS", ""),
        ("XDG_RUNTIME_DIR", ""),
    ]);
    assert_eq!(dirs.config_home, p("/home/u/.config"));
    assert_eq!(dirs.config_dirs, paths(&["/etc/xdg"]));
    assert_eq!(dirs.data_dirs, paths(&["/usr/local/share/", "/usr/share/"]));
    assert_eq!(dirs.runtime_dir, None);
}

#[test]
fn the_search_lists_keep_their_order_and_drop_invalid_entries() {
    let dirs = xdg(&[
        ("HOME", "/home/u"),
        ("XDG_DATA_DIRS", "/opt/share:relative::/usr/share"),
        ("XDG_CONFIG_DIRS", "/etc/site:/etc/xdg"),
    ]);
    assert_eq!(dirs.data_dirs, paths(&["/opt/share", "/usr/share"]));
    assert_eq!(dirs.config_dirs, paths(&["/etc/site", "/etc/xdg"]));
}

#[test]
fn a_search_list_of_only_invalid_entries_is_empty_not_the_default() {
    let dirs = xdg(&[("HOME", "/home/u"), ("XDG_DATA_DIRS", "a:b")]);
    assert_eq!(dirs.data_dirs, Vec::<PathBuf>::new());
}

#[test]
fn no_home_is_an_error_on_xdg() {
    let err = BaseDirs::resolve(Platform::Xdg, &env(&[])).unwrap_err();
    assert_eq!(
        err,
        Error::NoHome {
            platform: Platform::Xdg
        }
    );
    assert!(err.to_string().contains("HOME"), "{err}");
}

#[test]
fn a_relative_home_counts_as_unset() {
    let err = BaseDirs::resolve(Platform::Xdg, &env(&[("HOME", "u")])).unwrap_err();
    assert_eq!(
        err,
        Error::NoHome {
            platform: Platform::Xdg
        }
    );
}

#[test]
fn xdg_variables_resolve_without_home_when_all_five_are_set() {
    // HOME is only needed for a default; a fully specified environment has none to compute.
    let dirs = BaseDirs::resolve(
        Platform::Xdg,
        &env(&[
            ("XDG_CONFIG_HOME", "/c"),
            ("XDG_DATA_HOME", "/d"),
            ("XDG_STATE_HOME", "/s"),
            ("XDG_CACHE_HOME", "/k"),
        ]),
    )
    .expect("no default needed");
    assert_eq!(dirs.config_home, p("/c"));
    assert_eq!(dirs.cache_home, p("/k"));
}

// --- macOS: the Library layout of the File System Programming Guide ---

#[test]
fn macos_defaults_are_the_library_layout() {
    let dirs = BaseDirs::resolve(Platform::MacOs, &env(&[("HOME", "/Users/u")])).unwrap();
    assert_eq!(dirs.config_home, p("/Users/u/Library/Application Support"));
    assert_eq!(dirs.data_home, p("/Users/u/Library/Application Support"));
    assert_eq!(dirs.state_home, p("/Users/u/Library/Application Support"));
    assert_eq!(dirs.cache_home, p("/Users/u/Library/Caches"));
    assert_eq!(dirs.runtime_dir, None);
    assert_eq!(dirs.config_dirs, paths(&["/Library/Application Support"]));
    assert_eq!(dirs.data_dirs, paths(&["/Library/Application Support"]));
}

#[test]
fn macos_runtime_is_the_per_user_tmpdir_when_absolute() {
    let with = BaseDirs::resolve(
        Platform::MacOs,
        &env(&[("HOME", "/Users/u"), ("TMPDIR", "/var/folders/xx/T/")]),
    )
    .unwrap();
    assert_eq!(with.runtime_dir, Some(p("/var/folders/xx/T/")));
    let relative = BaseDirs::resolve(
        Platform::MacOs,
        &env(&[("HOME", "/Users/u"), ("TMPDIR", "tmp")]),
    )
    .unwrap();
    assert_eq!(relative.runtime_dir, None);
}

#[test]
fn an_absolute_xdg_variable_wins_on_macos_too() {
    let dirs = BaseDirs::resolve(
        Platform::MacOs,
        &env(&[
            ("HOME", "/Users/u"),
            ("XDG_CONFIG_HOME", "/Users/u/.config"),
            ("XDG_CACHE_HOME", "relative"),
            ("XDG_CONFIG_DIRS", "/etc/xdg"),
        ]),
    )
    .unwrap();
    assert_eq!(dirs.config_home, p("/Users/u/.config"));
    assert_eq!(dirs.cache_home, p("/Users/u/Library/Caches"));
    assert_eq!(dirs.config_dirs, paths(&["/etc/xdg"]));
}

#[test]
fn no_home_is_an_error_on_macos() {
    let err = BaseDirs::resolve(Platform::MacOs, &env(&[("TMPDIR", "/tmp")])).unwrap_err();
    assert_eq!(
        err,
        Error::NoHome {
            platform: Platform::MacOs
        }
    );
}

// --- Windows: the known folders ---

#[test]
fn windows_defaults_are_the_known_folders() {
    let dirs = BaseDirs::resolve(
        Platform::Windows,
        &env(&[
            ("APPDATA", r"C:\Users\u\AppData\Roaming"),
            ("LOCALAPPDATA", r"C:\Users\u\AppData\Local"),
            ("ProgramData", r"C:\ProgramData"),
        ]),
    )
    .unwrap();
    assert_eq!(dirs.config_home, p(r"C:\Users\u\AppData\Roaming"));
    assert_eq!(dirs.data_home, p(r"C:\Users\u\AppData\Roaming"));
    assert_eq!(dirs.state_home, p(r"C:\Users\u\AppData\Local"));
    assert_eq!(dirs.cache_home, p(r"C:\Users\u\AppData\Local"));
    assert_eq!(dirs.runtime_dir, None);
    assert_eq!(dirs.config_dirs, paths(&[r"C:\ProgramData"]));
    assert_eq!(dirs.data_dirs, paths(&[r"C:\ProgramData"]));
}

#[test]
fn windows_falls_back_to_userprofile_and_the_documented_defaults() {
    let dirs =
        BaseDirs::resolve(Platform::Windows, &env(&[("USERPROFILE", r"D:\Home\u")])).unwrap();
    assert_eq!(dirs.config_home, p(r"D:\Home\u\AppData\Roaming"));
    assert_eq!(dirs.cache_home, p(r"D:\Home\u\AppData\Local"));
    assert_eq!(dirs.config_dirs, paths(&[r"C:\ProgramData"]));
}

#[test]
fn windows_judges_absoluteness_by_drive_or_unc_prefix() {
    let dirs = BaseDirs::resolve(
        Platform::Windows,
        &env(&[
            ("USERPROFILE", r"C:\Users\u"),
            ("APPDATA", r"Roaming"),
            ("LOCALAPPDATA", r"\\server\share\u"),
            ("ProgramData", "/ProgramData"),
        ]),
    )
    .unwrap();
    assert_eq!(dirs.config_home, p(r"C:\Users\u\AppData\Roaming"));
    assert_eq!(dirs.cache_home, p(r"\\server\share\u"));
    assert_eq!(dirs.config_dirs, paths(&[r"C:\ProgramData"]));
}

#[test]
fn windows_without_a_profile_is_an_error() {
    let err = BaseDirs::resolve(Platform::Windows, &env(&[("HOME", "/home/u")])).unwrap_err();
    assert_eq!(
        err,
        Error::NoHome {
            platform: Platform::Windows
        }
    );
    assert!(err.to_string().contains("USERPROFILE"), "{err}");
}

#[test]
fn an_absolute_xdg_variable_wins_on_windows_too() {
    let dirs = BaseDirs::resolve(
        Platform::Windows,
        &env(&[
            ("USERPROFILE", r"C:\Users\u"),
            ("XDG_CONFIG_HOME", r"C:\cfg"),
        ]),
    )
    .unwrap();
    assert_eq!(dirs.config_home, p(r"C:\cfg"));
    assert_eq!(dirs.data_home, p(r"C:\Users\u\AppData\Roaming"));
}

// --- the suite's layout under the bases ---

#[test]
fn an_app_id_is_a_component_name() {
    assert_eq!(AppId::new("paths").unwrap().component(), "paths");
    assert_eq!(
        AppId::new("docs-aggregate").unwrap().component(),
        "docs-aggregate"
    );
    for bad in [
        "",
        "Paths",
        "docs_aggregate",
        "-paths",
        "paths-",
        "a--b",
        "pub/paths",
    ] {
        assert_eq!(
            AppId::new(bad).unwrap_err(),
            Error::InvalidAppId(bad.to_string()),
            "{bad:?}"
        );
    }
}

#[test]
fn the_xdg_layout_is_one_suite_directory_per_base() {
    let app = xdg(&[("HOME", "/home/u"), ("XDG_RUNTIME_DIR", "/run/user/1000")])
        .for_app(&AppId::new("paths").unwrap());
    assert_eq!(app.config_dir, p("/home/u/.config/public-software/paths"));
    assert_eq!(
        app.data_dir,
        p("/home/u/.local/share/public-software/paths")
    );
    assert_eq!(
        app.state_dir,
        p("/home/u/.local/state/public-software/paths")
    );
    assert_eq!(app.cache_dir, p("/home/u/.cache/public-software/paths"));
    assert_eq!(
        app.runtime_dir,
        Some(p("/run/user/1000/public-software/paths"))
    );
    assert_eq!(app.config_dirs, paths(&["/etc/xdg/public-software/paths"]));
    assert_eq!(
        app.data_dirs,
        paths(&[
            "/usr/local/share/public-software/paths",
            "/usr/share/public-software/paths"
        ])
    );
}

#[test]
fn the_macos_layout_is_a_bundle_identifier() {
    let app = BaseDirs::resolve(Platform::MacOs, &env(&[("HOME", "/Users/u")]))
        .unwrap()
        .for_app(&AppId::new("docs-aggregate").unwrap());
    assert_eq!(
        app.config_dir,
        p("/Users/u/Library/Application Support/dev.publicsoftware.docs-aggregate")
    );
    assert_eq!(
        app.cache_dir,
        p("/Users/u/Library/Caches/dev.publicsoftware.docs-aggregate")
    );
    assert_eq!(
        app.config_dirs,
        paths(&["/Library/Application Support/dev.publicsoftware.docs-aggregate"])
    );
}

#[test]
fn the_windows_layout_is_vendor_then_app() {
    let app = BaseDirs::resolve(Platform::Windows, &env(&[("USERPROFILE", r"C:\Users\u")]))
        .unwrap()
        .for_app(&AppId::new("paths").unwrap());
    assert_eq!(
        app.config_dir,
        p(r"C:\Users\u\AppData\Roaming\Public Software\paths")
    );
    assert_eq!(
        app.cache_dir,
        p(r"C:\Users\u\AppData\Local\Public Software\paths")
    );
    assert_eq!(app.runtime_dir, None);
    assert_eq!(
        app.config_dirs,
        paths(&[r"C:\ProgramData\Public Software\paths"])
    );
}

#[test]
fn the_search_order_is_home_first_then_the_lists_in_order() {
    let app = xdg(&[
        ("HOME", "/home/u"),
        ("XDG_CONFIG_DIRS", "/etc/site:/etc/xdg"),
    ])
    .for_app(&AppId::new("paths").unwrap());
    let config: Vec<&Path> = app.config_search().collect();
    assert_eq!(
        config,
        vec![
            Path::new("/home/u/.config/public-software/paths"),
            Path::new("/etc/site/public-software/paths"),
            Path::new("/etc/xdg/public-software/paths"),
        ]
    );
    let data: Vec<&Path> = app.data_search().collect();
    assert_eq!(
        data[0],
        Path::new("/home/u/.local/share/public-software/paths")
    );
    assert_eq!(data.len(), 3);
}

#[test]
fn a_trailing_separator_on_a_base_is_not_doubled() {
    let app = xdg(&[("HOME", "/home/u/"), ("XDG_CONFIG_HOME", "/cfg/")])
        .for_app(&AppId::new("paths").unwrap());
    assert_eq!(app.config_dir, p("/cfg/public-software/paths"));
    assert_eq!(
        app.data_dir,
        p("/home/u/.local/share/public-software/paths")
    );
}

// --- the edges: the process environment and the host platform ---

#[test]
fn the_process_environment_is_one_env_among_others() {
    // Names are kept as the OS reports them (Windows spells this one `Path`), so the lookup that
    // applies the platform's rule is the one to compare with the standard library's.
    let live = Env::from_process();
    assert_eq!(
        live.lookup(Platform::current(), "PATH"),
        std::env::var_os("PATH").as_deref()
    );
    let fixed = env(&[("HOME", "/home/u")]);
    assert_eq!(
        fixed.get("HOME").map(|v| v.to_os_string()),
        Some("/home/u".into())
    );
    assert_eq!(fixed.get("XDG_CONFIG_HOME"), None);
}

#[test]
fn the_current_platform_matches_the_target() {
    let expected = if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Xdg
    };
    assert_eq!(Platform::current(), expected);
    assert_eq!(
        BaseDirs::resolve(expected, &env(&[("HOME", "/h"), ("USERPROFILE", r"C:\u")]))
            .unwrap()
            .platform,
        expected
    );
}

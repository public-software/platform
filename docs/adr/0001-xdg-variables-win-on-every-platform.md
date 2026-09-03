# ADR-0001: The XDG variables win on every platform; the native folders are the defaults

- Status: accepted
- Date: 2026-09-03
- Scope: this repository only (cross-repo decisions are RFCs in public-software/rfcs)

## Context

Every program of the suite needs the same answer to "where do my configuration, data, state, cache and
runtime files go", on Linux, macOS and Windows. The XDG Base Directory Specification answers it for the
Unix-like desktops with seven environment variables and their defaults; Apple documents the `Library` layout
(`Application Support`, `Caches`; `Preferences` is reserved for the defaults system) and Microsoft the known
folders (`FOLDERID_RoamingAppData`, `FOLDERID_LocalAppData`, `FOLDERID_ProgramData`). The existing
permissively licensed crates in this space differ on whether an `XDG_*` variable is honoured on macOS and
Windows at all, and they read the process environment themselves, which makes every table hard to test and
impossible to test off its own host.

## Decision

`pub-platform-paths` resolves the directories as one function of a `Platform` and an explicit `Env`:

1. On every platform, a specification variable set to an absolute path wins (`XDG_CONFIG_HOME`,
   `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`, `XDG_RUNTIME_DIR`, `XDG_CONFIG_DIRS`,
   `XDG_DATA_DIRS`). Unset, empty or relative, it takes the platform's default, as the specification says
   ("consider the path invalid and ignore it").
2. The defaults are the native folders: `$HOME/.config` and friends on XDG platforms;
   `~/Library/Application Support` and `~/Library/Caches` on macOS, with `$TMPDIR` as the runtime directory
   because the system creates it per user with mode `0700`; `%APPDATA%`, `%LOCALAPPDATA%` and
   `%ProgramData%` on Windows, with the documented `%USERPROFILE%\AppData\...` and `C:\ProgramData`
   fallbacks and no runtime directory, since nothing there carries the specification's guarantees.
3. The search lists split on `:` as the specification writes them, and on `;` on Windows, where `:` is part
   of every drive letter.
4. The suite's own directory under each base is `public-software/<component>` on XDG platforms,
   `dev.publicsoftware.<component>` on macOS (the bundle-identifier convention Apple documents) and
   `Public Software\<component>` on Windows (vendor, then application).
5. Nothing in the crate reads the process environment or touches the disk; `Env::from_process()` is the one
   edge a program calls, and absoluteness and joining are judged per target platform from the value's bytes,
   never with the host's `Path` rules.

## Consequences

- Every table is exercised on every host, so the CI matrix (Linux, macOS, Windows) proves the same behaviour
  three times rather than one third each.
- A user or a test can relocate every directory of every suite program with the seven variables, on any
  platform, without the program knowing.
- A list set to only invalid entries is empty, not the default: the literal reading of the specification.
  A caller that wants the default for that case checks for emptiness.
- The crate reports `runtime_dir: None` rather than inventing a fallback; the specification tells the
  application to fall back and warn, which is the caller's decision.
- The later `pub-platform-config` reads its layers through `AppDirs::config_search()`, most important
  first, and needs no path logic of its own.

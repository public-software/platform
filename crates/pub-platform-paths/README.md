# pub-platform-paths

The `paths` library of [platform](https://github.com/public-software/platform), part of Public Software. Kind: `lib`.

Where a program of the suite keeps its files. It resolves the base directories of the
[XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/) (configuration, data,
state, cache, runtime, and the two system-wide search lists) for Linux, macOS and Windows from an explicit
environment map, and the suite's own directory for each component under them (`public-software/<component>`,
`dev.publicsoftware.<component>`, `Public Software\<component>`). One rule on every platform: an `XDG_*`
variable set to an absolute path wins; otherwise the native folders are the defaults (see
`docs/adr/0001-xdg-variables-win-on-every-platform.md`). Nothing reads the process environment unless asked
(`Env::from_process()`), nothing touches the disk, and no directory is created: that is the caller's job. It has
no dependency.

```rust
use pub_platform_paths::{AppId, BaseDirs, Env, Platform};

let dirs = BaseDirs::resolve(Platform::current(), &Env::from_process())?;
let app = dirs.for_app(&AppId::new("docs-aggregate")?);
// app.config_dir, app.data_dir, app.state_dir, app.cache_dir, app.runtime_dir (Option),
// app.config_search() and app.data_search(), most important first
```

```sh
cargo nextest run -p pub-platform-paths
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "pub-platform-paths"
kind      = "lib"
ledger    = "paths"
readiness = "partial"
effort    = 2
specs     = ["xdg-basedir-0.8"]
provides  = []
requires  = []
```

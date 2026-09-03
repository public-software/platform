# Provenance

This repository is a spec-first cleanroom implementation. Record here what was consulted.

## Specifications used
- XDG Base Directory Specification, version 0.8 (2021-05-08), freedesktop.org
  (https://specifications.freedesktop.org/basedir/latest/): the seven variables, their defaults, the
  unset-or-empty rule, the absolute-path rule ("consider the path invalid and ignore it"), the precedence of the
  home directories over the search lists and of a list's order, and the runtime directory's guarantees. The
  specification `pub-platform-paths` implements.

## Behavioural references (cited, not copied)
- Apple, File System Programming Guide, "macOS Library Directory Details"
  (https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/MacOSXDirectories/MacOSXDirectories.html,
  Apple documentation): the purpose of `Library/Application Support` and `Library/Caches`, the
  bundle-identifier subdirectory convention, and the rule never to write to `Library/Preferences` yourself; the
  macOS defaults of `pub-platform-paths`.
- Microsoft, Windows App Development, "KNOWNFOLDERID"
  (https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid, Microsoft documentation):
  `FOLDERID_RoamingAppData` (`%APPDATA%`, `%USERPROFILE%\AppData\Roaming`), `FOLDERID_LocalAppData`
  (`%LOCALAPPDATA%`, `%USERPROFILE%\AppData\Local`) and `FOLDERID_ProgramData` (`%ProgramData%`,
  `C:\ProgramData`), and which of them roams; the Windows defaults of `pub-platform-paths`.
- The `dirs` and `directories` crates (MIT OR Apache-2.0) were not opened; the specification and the two vendor
  documents above are the whole reference.

## Copyleft sources
None consulted. Contributors who have studied GPL/AGPL implementations of this domain do not author the corresponding modules (two-team rule; see the Charter §09).

## AI assistance
Prompts point at the specifications and conformance suites above, never at copyleft source. Generated code is reviewed against this list before merge.

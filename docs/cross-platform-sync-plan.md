# Cross-platform save synchronization plan

This document summarizes the context and proposed implementation plan for
making Ludusavi backups portable across operating systems, with a first focus
on Windows paths and Wine/Proton prefixes.

Related issues:

- https://github.com/mtkennerly/ludusavi/issues/156
- https://github.com/mtkennerly/ludusavi/issues/194
- https://github.com/mtkennerly/ludusavi/issues/310
- https://github.com/mtkennerly/ludusavi/issues/490

Related documentation:

- `docs/help/backup-structure.md`
- `docs/help/backup-validation.md`
- `docs/help/redirects.md`
- `docs/help/roots.md`
- `docs/help/transfer-between-operating-systems.md`

## Goal

Ludusavi should be able to back up on any supported platform and restore on any
supported platform whenever the save locations are semantically equivalent.
Machine-specific details such as the source operating system username, SteamOS'
`deck` account, a Wine username such as `steamuser`, or the absolute location of
a Wine prefix should not define the identity of the save data.

The core principle is:

> A backup should identify the semantic save location, not the source machine's
> physical path.

For example, these paths should be treated as the same logical save location:

```text
C:/Users/Alice/Documents/Remedy/Alan Wake/save.dat
/home/deck/Games/Heroic/Prefixes/Alan Wake/drive_c/users/steamuser/Documents/Remedy/Alan Wake/save.dat
```

Both represent the Windows "Documents" save location for the game. The username
and Wine prefix only describe where that semantic location happens to live on a
specific machine.

## Current behavior

Today, backups are keyed by rendered paths after redirects are applied.
`ScannedFile::mapping_key` stores `effective(scan_key).render()`, and backup
planning writes that key directly into `mapping.yaml`. The simple backup format
then converts the rendered path into `drive-*` folders for on-disk storage.

This is safe for same-machine restore, but it couples the backup identity to
the machine that created it. A Wine save discovered on Linux may be stored under
a path like:

```text
/home/deck/Games/Heroic/Prefixes/Alan Wake/drive_c/users/steamuser/Documents/...
```

That path is not useful as the canonical identity of the save. It includes:

- the Linux user's home directory
- the launcher's prefix layout
- the game-specific prefix folder
- the Wine user name

The prefix is useful while scanning and restoring on Linux, but it should not be
part of the portable backup key.

## Issue context

### #156: Relative paths instead of absolute paths

Issue #156 proposed storing paths relative to save roots or manifest entries.
The discussion identified several hard cases:

- one backup may include files from multiple system users;
- special folders such as Documents or XDG directories can be relocated;
- the same game can be installed through multiple stores;
- native Windows and native Linux paths are not always equivalent;
- the manifest usually lists folders, not file-level cross-OS relationships.

The important conclusion is that fully relative, cross-OS restoration is not
reliable without more semantic information. However, the later discussion
introduced a narrower and more reliable idea: a Windows game running through
Wine should be backed up as a Windows save, not as a Linux save whose path
happens to contain `drive_c`.

### #194: Translate Wine prefixes across OSes

Issue #194 focuses on Windows/Wine portability. The original proposal suggested
adding backup metadata such as `os` and `wine_prefixes`, then translating paths
that live under known Wine prefixes.

The discussion later clarified several user pain points:

- backing up in Wine on Linux and restoring on Windows should not require a
  per-game redirect;
- backing up on Windows and restoring into Wine on Linux needs a way to choose
  the target prefix;
- current game-specific Wine prefix roots help scanning, but they do not solve
  cross-platform path identity;
- redirects from a whole prefix root to `C:/Users/<name>` do not work when the
  prefix path itself is still retained in the backup key;
- users currently need fully game-specific redirects down to
  `.../drive_c/users/<wine-user>` to get a clean Windows-style result.

This plan treats #194 as the primary implementation target.

### #310: Complex Wine prefixes across installs

Issue #310 describes a different but related problem: redirects only replace
path prefixes. They do not handle variable path segments after the game
directory, such as:

```text
/home/dane/game/drive_c/users/dane/saves
/home/deck/game/drive_c/users/deck/saves
```

Regex redirects with capture groups would help advanced users normalize these
paths. That is useful, but it is not the best first fix for cross-platform
restore. Regex redirects still require users to encode machine-specific paths.
The more direct solution is to remove usernames and Wine prefix locations from
the backup identity in the first place.

Regex redirects should remain a follow-up enhancement, not the foundation of
the cross-platform model.

### #490: Foreign-platform saves

Issue #490 asks for automatically de-selecting saves from another platform.
That request is a symptom of the same underlying issue: Ludusavi can currently
see saves from another OS in the same backup set, but it does not know whether
they are portable, foreign, redundant, or dangerous to remove.

Semantic path identity gives Ludusavi a better basis for warnings. If two
physical paths map to the same semantic location, they are cross-platform
counterparts. If they do not, Ludusavi can keep warning or require user action.

## Pain point coverage

| Source | Pain point | Coverage | Notes |
| --- | --- | --- | --- |
| #194 | Wine backup to Windows restore without per-game redirects | Phase 1 and Phase 3 | Directly addressed by storing Windows/Wine saves under the same semantic key. |
| #194 | Windows backup to Wine restore needs a target prefix | Phase 3 | Requires deterministic prefix selection or an explicit per-game choice. |
| #194 | SteamOS `deck`, Wine `steamuser`, and Windows usernames should not affect backup keys | User identity model | Covered for current-user saves; multi-user scans must stay explicit. |
| #194 | Registry transfer between Windows and Wine | Registry model | Explicitly deferred, but the format should reserve registry metadata now. |
| #310 | Variable path segments such as `drive_c/users/<name>` | Semantic paths | Solved by removing the user segment from the backup identity. |
| #310 | Regex redirects | Phase 5 | Kept as an advanced feature after semantic paths exist. |
| transfer docs | Native Windows and native Linux paths may not be equivalent | Phase 6 | Not solved by Phase 1 unless manifest metadata proves equivalence. |
| transfer docs | Relocated Windows KnownFolders and changed XDG paths | Phase 3 | Restore must materialize semantic paths through current-platform location APIs where available. |
| #490 | Foreign-platform backup entries | Phase 4 | Semantic comparison should expose explicit match and mismatch signals. |

## Design principle

Separate the three meanings that are currently collapsed into one path string:

1. Physical path
   The actual file path used for reading or writing on the current machine.

2. Semantic path
   The portable identity of the save location, independent of username,
   Wine prefix location, or current OS.

3. Storage path
   The safe path used inside Ludusavi's backup folder or zip archive.

For normal legacy paths, these may still line up closely. For Windows/Wine
paths, they must be distinct.

Example:

```text
Physical path:
  /home/deck/Games/Heroic/Prefixes/Alan Wake/drive_c/users/steamuser/Documents/Remedy/Alan Wake/save.dat

Semantic path:
  <winDocuments>/Remedy/Alan Wake/save.dat

Storage path:
  __ludusavi_semantic__/winDocuments/Remedy/Alan Wake/save.dat
```

On Windows, the same semantic path would materialize to the current user's
Documents folder. In Wine, it would materialize to the chosen prefix's
`drive_c/users/<wine-user>/Documents` folder.

## Proposed semantic paths

Use existing manifest-style placeholders where possible because several of them
already represent semantic locations. Add new tokens only where the current
placeholder set cannot describe a common Windows location.

- `<winDocuments>`
- `<winAppData>`
- `<winLocalAppData>`
- `<winLocalAppDataLow>`
- `<winPublic>`
- `<winProgramData>`
- `<winDir>`
- `<xdgData>`
- `<xdgConfig>`
- `<home>`
- `<base>`
- `<root>`

Proposed new semantic tokens:

- `<winHome>`
- `<winSavedGames>`

For the first implementation phase, only Windows semantic locations should be
used for Windows and Wine equivalence. Native Linux/macOS equivalence can be
added later when Ludusavi has enough information to prove that two manifest
locations are counterparts.

Examples:

```text
Windows physical:
  C:/Users/Alice/AppData/Roaming/Game/save.dat
Semantic:
  <winAppData>/Game/save.dat

Wine physical:
  /home/deck/Prefixes/Game/drive_c/users/steamuser/AppData/Roaming/Game/save.dat
Semantic:
  <winAppData>/Game/save.dat

Windows physical:
  C:/ProgramData/Game/save.dat
Semantic:
  <winProgramData>/Game/save.dat
```

Usernames are intentionally absent from these semantic paths for normal
current-user saves.

## Semantic key syntax

Semantic keys are serialized strings in `mapping.yaml`, but they are not OS
paths. They must be parsed as semantic keys before use.

Canonical rules:

- use `/` as the only separator;
- begin with a recognized semantic base token, such as `<winDocuments>` or
  `<steamUserdata>`;
- reject `.` and `..` path components after parsing;
- preserve the display casing of the discovered tail;
- compare keys according to the semantic base's case policy;
- never pass a semantic key directly to `StrictPath` or filesystem APIs.

Initial case policy:

- Windows/Wine semantic bases are case-insensitive for equality and conflict
  detection, but preserve casing for display and storage;
- Steam userdata should use the manifest/store policy for the matched entry;
- future native Linux/macOS bases should keep their platform case policy.

If two distinct physical files produce semantic keys that differ only by case in
a case-insensitive namespace, treat that as a semantic-key conflict. Do not
choose one file silently. This covers cases where a native Linux game has
case-distinct files that would collide on Windows.

Storage-path generation should encode semantic keys through a single structured
function rather than by string concatenation. That function owns escaping,
reserved component handling, and case-collision checks.

## Semantic key sources

Semantic keys should come from explicit source information, not from filename
similarity.

There are two primary sources.

### Source precedence

Semantic key derivation must be deterministic:

1. manifest-derived keys take precedence whenever a scan candidate has manifest
   origin metadata;
2. platform-location reverse mapping is used only when a path has no usable
   manifest origin;
3. if neither source can produce a semantic key, the file stays under legacy
   physical-path behavior.

This avoids different code paths producing different keys for the same file.
For example, a Steam userdata path should use a manifest/store-derived
`<steamUserdata>` key instead of being reverse-mapped as a generic path under a
Windows user profile.

### Manifest-derived keys

When a scan path comes from a manifest entry, Ludusavi should keep enough origin
metadata to reconstruct the matched placeholder path and the tail below it. The
manifest entry is often already the best semantic description of the save.
However, placeholders are not all equally portable. A semantic key derived from
`<base>` or `<root>` must include the store/root identity when that identity is
needed to distinguish two installs of the same game. Steam and GOG install
folder saves should not collapse into one key merely because both came from a
manifest entry ending in the same file name.

Stable root identity should come from durable metadata, not from the host path.
Examples:

- Steam: store kind plus Steam app ID or shortcut identity, with Steam root
  omitted from the semantic key;
- GOG/Epic/Heroic/Lutris: store kind plus store game ID when available;
- generic `Other` roots: only use a manifest-derived portable key when the
  configured root has a stable user-visible identity; otherwise keep legacy
  physical-path behavior for that entry.

When no stable root identity exists, do not invent one from the absolute root
path. That would reintroduce the machine-specific coupling this design removes.

When multiple manifest entries match the same physical file, choose the most
specific entry, defined as the longest matched physical prefix after placeholder
expansion. If two entries have the same matched length, break ties by manifest
declaration order. The same rule must be used in backup preview, backup, and
restore planning.

For example:

```yaml
files:
  "<winDocuments>/Remedy/Alan Wake": {}
```

If the physical file is:

```text
/home/deck/Prefixes/Alan Wake/drive_c/users/steamuser/Documents/Remedy/Alan Wake/save.dat
```

then the semantic key should be:

```text
<winDocuments>/Remedy/Alan Wake/save.dat
```

This also matters for store-specific paths that are not Windows KnownFolders.
Steam `userdata` is the most important early case. Windows and Linux Steam paths
often differ only by the Steam root:

```text
C:/Program Files (x86)/Steam/userdata/<storeUserId>/<storeGameId>/remote/save.dat
/home/deck/.local/share/Steam/userdata/<storeUserId>/<storeGameId>/remote/save.dat
```

These should be modeled with a store-specific semantic base, for example:

```text
<steamUserdata>/<storeUserId>/<storeGameId>/remote/save.dat
```

`<storeUserId>` is intentionally part of the semantic key. Steam account A's
save and Steam account B's save are different semantic locations even when they
belong to the same game on the same machine. This is different from OS usernames
and Wine usernames, which are host materialization details for current-user
profile saves.

This is not the same as native Windows/Linux path guessing. It is valid because
the Steam userdata structure and manifest placeholders prove that the paths are
the same store-owned save namespace.

Manifest changes only affect future scans. Existing backup keys are recorded as
strings in `mapping.yaml` and must not be retroactively rewritten by re-parsing
old backups against a newer manifest. Differential comparison should compare the
stored old semantic key to the newly scanned semantic key.

Implementation implication: scan candidates should carry an origin record such
as the manifest path, root/store kind, expanded placeholders, matched prefix,
and matched tail. A plain `HashSet<StrictPath>` is not enough once semantic keys
are introduced.

### Platform-location reverse mapping

When manifest origin metadata is not enough, Ludusavi can reverse-map physical
Windows or Wine paths into known semantic bases:

- current Windows KnownFolders to `<winDocuments>`, `<winAppData>`,
  `<winLocalAppData>`, `<winLocalAppDataLow>`, `<winSavedGames>`, and related
  bases;
- Wine prefix paths under `drive_c/users/<wine-user>/...` to the corresponding
  Windows semantic bases;
- Wine `ProgramData`, `Windows`, and drive roots to `<winProgramData>`,
  `<winDir>`, or `WinDrive(char)`.

Reverse mapping must be explicit and ordered. Prefer more specific bases before
broader ones:

1. `Saved Games`
2. `Documents` and `My Documents`
3. `AppData/LocalLow`
4. `AppData/Local` and `Local Settings/Application Data`
5. `AppData/Roaming` and `Application Data`
6. `Public`
7. `ProgramData`
8. `Windows`
9. current-user home
10. drive root

`<winHome>` means the current Windows user profile root after all more specific
KnownFolder bases have failed to match. For example,
`C:/Users/Alice/MyGames/save.dat` may become `<winHome>/MyGames/save.dat`, but
`C:/Users/Alice/Documents/Game/save.dat` must become `<winDocuments>/Game/save.dat`
when Documents resolves to that location.

When reverse-mapping to `<winHome>`, the first tail component should not be a
known folder alias such as `Documents`, `My Documents`, `AppData`, `Application
Data`, `Local Settings`, `Saved Games`, or `Desktop` unless the relevant
KnownFolder check already proved that the directory is not that semantic
location on the current machine. This keeps the broad user-profile base from
swallowing paths that should have been handled by a more specific base.

Windows and Wine matching should be case-insensitive for these base segments.
This covers common Wine and legacy aliases such as `Application Data`,
`Local Settings/Application Data`, `My Documents`, and case variations like
`Appdata`.

Native Windows-to-native Linux equivalence must not be inferred from filename
or suffix similarity. It should require a manifest relationship or explicit
user configuration.

## User identity model

Most save data belongs to the current user profile. In that common case,
the original username is irrelevant:

- Windows backup from `C:/Users/Alice/...` should restore for user `Bob` on a
  different Windows machine.
- Windows backup from `C:/Users/Alice/...` should restore into
  `drive_c/users/steamuser/...` in a SteamOS/Wine prefix.
- SteamOS/Wine backup from `drive_c/users/steamuser/...` should restore into
  the current Windows user's profile on Windows.

Multi-user backups are a separate case and must be explicit. Ludusavi should
not silently encode multiple users through absolute paths. If a scan includes
save locations for multiple Windows users, the resulting semantic identity
should include an explicit profile identifier or require user selection.

The first implementation should support only the current-user profile for
semantic Windows/Wine keys. If multiple user profiles are found for one game in
one scan, Ludusavi should either:

- classify only the active/current profile semantically and keep other profiles
  under existing absolute-path behavior; or
- stop and ask the user to assign explicit profile identities.

It should not silently merge two user profiles into the same semantic key.

## Wine prefix model

Wine prefixes are restore targets, not backup identities.

When scanning a Wine prefix, Ludusavi needs the prefix path to find files. When
restoring into Wine, Ludusavi needs the prefix path to choose where files go.
However, the prefix path should not appear in the semantic backup key.

Prefix discovery should use the information Ludusavi already has:

- Steam Proton compatdata prefixes;
- Heroic game prefixes;
- Lutris prefixes;
- custom game `winePrefix` entries;
- the CLI `--wine-prefix` option;
- configured Wine prefix roots, including roots that use `<game>`.

macOS bottles from tools such as CrossOver or Whisky should use the same model
once their bottle path is normalized to a Wine-prefix-like directory. A bottle
is a prefix provider; it is not a separate backup identity.

First-version macOS bottle support may be limited to explicit bottle/prefix
paths supplied by the user. Automatic discovery for CrossOver
(`~/Library/Application Support/CrossOver/Bottles/<name>`) and Whisky
(`~/Library/Containers/com.isaacmarovitz.Whisky/Bottles/<uuid>` with metadata)
can be a follow-up task, but any explicit bottle path should still pass the same
prefix validation rules below.

### Prefix validation

Every discovered prefix should pass the same validation before it can
participate in semantic mapping. A valid prefix should have:

- a `drive_c` directory;
- at least one Wine state marker such as `system.reg`, `user.reg`, or
  `dosdevices`;
- a usable `drive_c/users` directory for current-user semantic paths, unless
  the path being handled is not user-profile based.

This prevents an ordinary game directory named `drive_c` from being treated as
a Wine prefix. Provider-specific discovery can be more permissive while looking
for candidates, but the semantic layer should only receive validated prefixes.

If a configured `custom_games[].winePrefix` entry fails validation, Ludusavi
should skip that source, continue with other valid sources, and report the
invalid prefix in logs and scan/preview output. A single bad custom prefix should
not block the whole game's backup unless it was the only source needed to
materialize a selected restore target.

### Wine user detection

The Wine user name is a materialization detail. It must not appear in the
semantic key for current-user saves.

When scanning, the user can be inferred from the matched path segment under
`drive_c/users/<wine-user>`. When restoring, Ludusavi should pick the Wine user
for the selected prefix with these rules:

1. use a per-game preferred Wine user if one is configured;
2. use the user directory that already contains the target save path;
3. use the only non-system user under `drive_c/users`;
4. for Steam Proton, prefer `steamuser` when present;
5. if multiple non-system users remain, require explicit user selection.

System user directories include `Public`, `Default`, `Default User`,
`All Users`, and other non-profile entries. Matching should be
case-insensitive.

### Symlinks and lexical paths

Semantic detection should be based on the Windows-view path through the prefix,
not on the final `realpath`.

Wine often represents folders such as Documents with symlinks. If Ludusavi
resolves the symlink first, a file reached through:

```text
<prefix>/drive_c/users/steamuser/Documents/Game/save.dat
```

may appear to live under the Linux home directory instead, which loses the Wine
context. The scanner should preserve the lexical path by which the file was
matched. Content operations may use an interpreted or canonical path, but
semantic derivation should use the prefix-relative alias path.

If a file is discovered through a symlink target outside the prefix, Ludusavi may
still map it semantically only when the scan candidate originated from a
validated prefix or a validated `dosdevices` mapping. It should not infer Wine
semantics from an arbitrary external path.

### DOS devices and non-C drives

Non-C drives are common in real Wine setups. A prefix may expose `D:` through
`dosdevices/d:` as a symlink to a path such as `/run/media/deck/HDD`.

`WinDrive(char)` should represent these paths without pretending they are under
`drive_c`. When backing up from Wine:

- paths under `<prefix>/drive_<letter>` map to `WinDrive(letter)`;
- paths under a target of `<prefix>/dosdevices/<letter>:` may map to
  `WinDrive(letter)` only when that dosdevice mapping belongs to the selected
  prefix context;
- paths under store roots such as Steam userdata should prefer the
  store-specific semantic base over a raw drive-root key.

When restoring a `WinDrive(char)` path:

- on Windows, write to that drive only if it exists or the user has configured a
  target for that drive;
- in Wine, write through the prefix's matching `dosdevices/<letter>:` mapping or
  `drive_<letter>` directory;
- if the drive is missing, fail with an actionable message asking the user to
  choose a drive/root mapping.

The implementation should not silently remap missing `D:` data to `C:`.

### Preferred prefix

When restoring a Windows semantic path on Linux, Ludusavi should choose a prefix
in this order:

1. the CLI `--wine-prefix` value for the current command;
2. a per-game preferred prefix saved in Ludusavi's configuration;
3. a game-specific prefix discovered from the launcher;
4. a custom game `winePrefix`;
5. a configured game-specific Wine root;
6. an explicit user selection.

If no single prefix can be identified, the restore should fail with a clear
ambiguity message. It should not guess.

`--wine-prefix` is a per-invocation override. If it is used for a command that
touches multiple games, it may apply only to games whose configured preferred
prefix is absent or consistent with the CLI prefix. If a game already has a
different preferred prefix, Ludusavi should fail for that game with an
actionable conflict message instead of silently overriding the saved preference.

The explicit selection should be stored as a per-game preferred prefix so that
the same ambiguity does not recur on every restore. GUI, CLI, and wrapper flows
should use the same stored preference.

The configuration should store preferred prefix data separately from backup
history. At minimum, it needs:

- game identity;
- preferred prefix path;
- optional preferred Wine user;
- optional drive mappings for non-C drives.

The game identity should use the same stable title/alias rules as existing
custom game settings. If a game is renamed through aliases, the preference must
follow the displayed game identity consistently.

## Registry model

Wine registry support is related but should not block file-based portability.

Current documentation says Wine prefix roots do not back up registry-based
saves from the prefix. Issue #194 also notes that proper registry transfer
would require parsing Wine `*.reg` files and translating them to or from the
native Windows registry.

The first implementation should explicitly focus on file-based saves. Registry
translation can be a later phase:

- Wine backup to Windows: parse relevant Wine `*.reg` entries and restore them
  to the Windows registry.
- Windows backup to Wine: export relevant Windows registry entries and merge
  them into the chosen prefix's `*.reg` files.

Until then, registry-based cross-platform transfers should be reported as
unsupported instead of being silently approximated.

Even before registry translation is implemented, the backup metadata should
reserve a registry format field so that registry support can be added without
redefining the file format again. For example:

```yaml
registryFormat: unsupported
```

Future values could distinguish native Windows registry exports from parsed
Wine registry files. The first semantic file-path implementation can keep this
field at `unsupported` for cross-platform registry transfer.

## Backup format

Introduce a versioned semantic path format in `mapping.yaml`, scoped to each
full backup chain.

Legacy backups without the new marker keep their existing behavior. New semantic
backups can store semantic file keys:

```yaml
backups:
  - name: "."
    os: linux
    pathFormat: semantic-v1
    registryFormat: unsupported
    files:
      "<winDocuments>/Remedy/Alan Wake/save.dat":
        hash: ...
        size: ...
```

The exact YAML field can be adjusted to fit existing serialization patterns,
but the invariant should be strict:

- semantic keys are not passed directly to `StrictPath`;
- they must be parsed as semantic paths first;
- they are materialized to physical paths only when scanning, restoring, or
  showing a current-machine target.

For the simple backup format, storage paths should be derived from semantic
paths using safe folder names. For example:

```text
__ludusavi_semantic__/winDocuments/Remedy/Alan Wake/save.dat
__ludusavi_semantic__/winAppData/Game/save.dat
__ludusavi_semantic__/winProgramData/Game/save.dat
```

The top-level `__ludusavi_semantic__` component is reserved inside semantic
backups. Literal game data with that name is still stored below a semantic base,
for example `__ludusavi_semantic__/winDrive-C/__ludusavi_semantic__/file.dat`,
so it cannot collide with the format marker. Validation should reject storage
paths that escape this namespace or mix semantic storage paths with legacy
`drive-*` paths in the same backup chain.

Storage paths must remain compatible with Windows long-path handling. The
semantic namespace adds path components, so backup and restore code should
verify the final storage/extracted path with the same long-path support used
elsewhere in Ludusavi. If a path cannot be represented safely on the current
filesystem, Ludusavi should report a clear error instead of truncating or
rewriting it.

For zip backups, the same storage path scheme can be used inside the archive.

### Differential chains and format switching

Each full backup starts a chain, and all differential children in that chain
must use the same `pathFormat` and `registryFormat` as the full backup.

When Ludusavi first switches a game from legacy absolute keys to
`semantic-v1`, it must create a new full backup. It should not create a
semantic differential child under a legacy full backup, because every key would
appear to be both removed and added. This avoids space waste and restore
ambiguity.

Old legacy chains remain readable and restorable. New semantic chains should not
rewrite legacy history.

The first switch to `semantic-v1` should surface a one-time notice in CLI and
GUI preview: a new full backup will be created, and the previous legacy chain
will remain restorable but frozen. This matters for retention settings because a
new full backup may change which older full chains are retained.

## Implementation phases

### Phase 1: Windows/Wine semantic file paths

Add a semantic path type, for example:

```rust
enum SavePath {
    Physical(StrictPath),
    Semantic(SemanticPath),
}
```

`SemanticPath` should represent the location category and relative tail:

```rust
struct SemanticPath {
    base: SemanticBase,
    tail: String,
}
```

Initial `SemanticBase` variants:

- `WinHome`
- `WinDocuments`
- `WinAppData`
- `WinLocalAppData`
- `WinLocalAppDataLow`
- `WinSavedGames`
- `WinPublic`
- `WinProgramData`
- `WinDir`
- `WinDrive(char)` for paths that are genuinely drive-rooted and not under a
  known special folder
- `SteamUserdata`
- manifest-derived store/root bases that are proven equivalent by the manifest
  origin metadata

Add conversion functions:

- physical Windows path to semantic path;
- physical Wine path under a known prefix to semantic path;
- manifest-origin scan candidate to semantic path;
- semantic path to physical Windows path for the current user;
- semantic path to physical Wine path for a selected prefix;
- semantic path to safe backup storage path.

This should not be implemented as broad recovery logic. Each semantic base must
have a direct, explicit mapping.

Windows materialization should use current-platform location APIs such as the
existing `CommonPath`/KnownFolder logic, not hardcoded
`C:/Users/<name>/Documents` paths. This preserves relocated Documents,
AppData, Saved Games, and similar folders.

For native Linux/macOS semantic keys that are introduced later, materialization
should likewise use the current XDG or platform-specific directory resolution
instead of reusing the source machine's expanded value.

Wine materialization cannot ask Windows for KnownFolders. It should write
through the selected prefix's Windows-view path, such as
`drive_c/users/<wine-user>/Documents`. If that directory is a symlink, the
filesystem will place the file at the symlink target while Ludusavi still keeps
the semantic identity as `<winDocuments>`.

### Phase 2: Preserve physical and semantic paths during scanning

Replace the current assumption that one `StrictPath` is enough to represent a
scanned file.

For each found file, keep:

- the physical path used to read file content;
- the semantic path used for backup identity, when one is available;
- the scan origin that explains why the semantic key is valid;
- the redirected path, if user redirects are still applied.

Redirects need a clear rule:

- legacy physical paths continue to use current redirect behavior;
- when a semantic key is available during backup, physical redirects do not
  change that semantic key;
- backup redirects may still be shown in preview for legacy paths, but they
  should not be needed to normalize usernames, Wine prefixes, or Steam roots
  once a semantic key exists;
- when restoring a semantic key, materialize it to the current machine's
  physical target first, then apply restore redirects to that physical target;
- restore preview should show all three values when relevant: semantic source,
  materialized physical target, and redirected final target;
- advanced semantic redirects are out of scope for the first implementation and
  require a separate design.

For Wine files, the backup key should be semantic. The copy source should remain
physical.

If the same semantic key is produced more than once during a single game scan,
handle it deterministically:

- if the entries are the same physical file reached through multiple aliases,
  collapse them into one entry;
- if the entries are distinct physical files, report a scan conflict for that
  semantic key instead of choosing one silently;
- while a semantic conflict exists, do not mark the prior backup entry for that
  semantic key as removed;
- if the conflict comes from overlapping roots, let the user disable one source
  or configure a more specific root.

This prevents a generic Heroic prefix root and a `<game>` Wine root from
silently racing to define the same backup key.

Backup preview should display both the physical source and the semantic key, for
example:

```text
/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat
  Stored as: <winDocuments>/Game/save.dat
```

This makes prefix misclassification visible before the user writes a backup.

### Selection, ignores, and toggles

Filtering has two layers:

- physical path ignores apply before semantic derivation, because they describe
  files on the current machine;
- game file selection, toggled backup paths, and restore selection should use
  the semantic key when one exists, because they describe the save identity.

Legacy toggles keyed by physical paths remain valid for legacy backups. When a
file first appears with a semantic key, preview should show the new semantic
identity and the old physical identity so users can confirm the selection. The
initial implementation should not silently migrate user toggles without showing
the changed key.

For restore previews, the selectable row should be keyed by the semantic source,
while the rendered target shows the materialized and redirected physical path.

### Phase 3: Restore materialization

When reading a semantic backup:

- on Windows, materialize Windows semantic paths to the current Windows user;
- on Linux/macOS with a selected Wine prefix, materialize Windows semantic paths
  into that prefix;
- without a selected prefix, report an actionable ambiguity;
- for legacy absolute backups, keep current restore behavior.

The UI and CLI preview should show both:

- the semantic source, such as `<winDocuments>/Game/save.dat`;
- the current physical target, such as
  `/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat`.

### Phase 4: Warnings and foreign-platform handling

Use semantic identity to expose explicit comparison signals for #490:

- `sameSemanticKey`: the current scan and existing backup describe the same
  save location, even if their physical paths differ;
- `sameNamespaceMissing`: an existing backup has entries in a semantic namespace
  that the current scan can understand, but no current entry matched them;
- `foreignNamespace`: an existing backup has entries in a namespace that the
  current platform cannot currently materialize;
- `ambiguousMaterialization`: Ludusavi understands the semantic key but cannot
  pick a single physical restore target.

Behavior should follow from those signals:

- `sameSemanticKey` entries are counterparts and can be compared normally;
- `sameNamespaceMissing` entries can be candidates for #490's optional
  de-selection or warning behavior;
- `foreignNamespace` entries should not be removed just because the current scan
  cannot see them;
- `ambiguousMaterialization` should block restore for that key until the user
  chooses a target.

### Phase 4.5: Preview and dry-run analysis

Before migration tooling exists, preview should expose what semantic mode would
do without writing anything:

- which legacy physical keys would become semantic keys;
- which games would start a new full backup chain;
- which files would land in foreign or ambiguous namespaces;
- which configured prefixes failed validation;
- which duplicate semantic keys would conflict.

This can start as CLI/GUI preview output rather than a separate migration
command. It gives users with large existing backup histories a way to understand
the change before enabling or relying on semantic backups.

### Phase 5: Regex redirects as an advanced feature

After semantic Windows/Wine paths are in place, add regex redirect support for
remaining advanced cases from #310.

This should be opt-in and explicit, for example:

```yaml
redirects:
  - kind: bidirectional
    mode: regex
    source: '^(.*/drive_c/users/)actual-user(/.*)$'
    target: '${1}standard-user${2}'
```

Regex redirects should not be required for normal Windows/Wine portability.

### Phase 6: Native cross-OS relationships

Native Windows, Linux, and macOS paths are harder than Windows/Wine paths
because semantic equivalence is not guaranteed by path shape alone. For example,
a native Windows path and a native Linux path may both contain the same file
name, but the manifest does not prove that the files are interchangeable.

Future work can add explicit manifest relationships, such as grouped path
variants:

```yaml
paths:
  - kind: folder
    variants:
      windows: "<winAppData>/Dustforce"
      linux: "<xdgData>/Dustforce"
      mac: "<home>/Library/Application Support/Hitbox Team/Dustforce"
```

Until that relationship data exists, the restore can consult the manifest at
restore time: when restoring a semantic key like `<<winDocuments>/Saved
Games/Hades/`, look up the game in the manifest, find the entry tagged with the
target OS, and materialize that path instead. This uses the manifest's existing
per-platform `when` constraints without any schema changes.

### Phase 7: Non-C drive fallback configuration

For `<<winDrive-d>>` semantic keys, add a `restore.driveMappings` config field:

```yaml
restore:
  driveMappings:
    d: /mnt/games
    e: /run/media/deck/SD
```

When materializing `<<winDrive-d>>/path/to/save` and no matching `dosdevices`
symlink exists, fall back to the configured mapping. If neither exists, raise
an actionable error suggesting the user configure a mapping.

### Out of scope for this work

The following are deliberately excluded so that this effort stays focused on the
core goal: portable backups between Windows and Wine/Proton without per-game
redirects. They can be pursued separately if there is demand:

- **Linux-native XDG bases** (`<xdgData>`, `<xdgConfig>`, etc.): equating a
  Windows known folder with a Linux XDG directory is the same unproven
  native-cross-OS equivalence that Phase 6 defers. Wine/Proton games already map
  to Windows bases through the prefix, so XDG bases are not needed here.
- **Steam userdata semantic identity / account mapping**: Steam Cloud
  `userdata/<id>/...` is a native Windows↔Linux concern, not a Wine/Proton one.
  It keeps its existing absolute-path (legacy) behavior, which restores
  correctly on the same account/machine.
- **Cross-platform registry translation**: backups continue to record registry
  data in the existing native format (`registryFormat` stays at its default).
  Translating between the Windows registry and Wine `*.reg` files is a separate
  effort.

## Problems solved

This plan solves or significantly improves:

- Windows to Wine restore without per-game redirects for user-profile saves;
- Wine to Windows restore without leaking `/home/deck/.../drive_c/...` into the
  backup identity;
- SteamOS `deck` and Wine `steamuser` usernames becoming irrelevant to backup
  compatibility;
- Windows usernames varying between machines;
- Heroic/Lutris/Proton prefixes living at different paths on different Linux
  machines;
- the common case where a Windows game stores saves under Documents, AppData,
  LocalAppData, LocalLow, Public, ProgramData, or the Windows directory;
- Wine prefixes and macOS bottles living at different host paths;
- non-C Wine drives when the matching `dosdevices` mapping exists, or via
  configurable `restore.driveMappings`;
- clearer detection of same semantic saves across platforms;
- safer warnings for foreign-platform backup entries;
- better cloud-sync and deduplication behavior, because the same semantic save
  can use the same backup storage path across machines instead of embedding
  each machine's username or prefix path.

## Problems not yet solved

- native Windows path to unrelated native Linux path equivalence (requires
  manifest consultation at restore time — see Phase 6);
- file-level matching by filename heuristics;
- multiple users in one backup without explicit user identity;
- multiple installed copies of the same game where the user wants distinct save
  streams;
- games whose Windows and Linux saves are structurally incompatible.

## Testing strategy

Add unit tests for semantic path conversion:

- semantic key parser rejects direct OS paths, `.` components, and `..`
  components;
- semantic key equality follows the base case policy;
- Windows current-user Documents to `<winDocuments>`;
- Windows AppData/Roaming to `<winAppData>`;
- Windows AppData/Local to `<winLocalAppData>`;
- relocated Windows KnownFolders materialize through the current location API;
- Wine `drive_c/users/steamuser/Documents` to `<winDocuments>`;
- Wine `drive_c/users/<custom-user>/AppData/Roaming` to `<winAppData>`;
- Wine XP-style aliases such as `Application Data`, `Local Settings`, and
  `My Documents`;
- Wine paths matched through symlinked Documents without losing prefix context;
- ProgramData and Windows directory paths;
- `dosdevices/d:` mappings to `WinDrive('d')`;
- Steam userdata paths to `<steamUserdata>`;
- paths outside known locations using explicit drive-root semantics.

Add scan tests:

- Wine prefix file scan stores semantic mapping key;
- physical copy source remains the Wine path;
- Windows scan and Wine scan of the same logical save produce the same mapping
  key;
- Windows Steam userdata and Linux Steam userdata produce the same semantic key;
- duplicate semantic keys from the same physical file are collapsed;
- duplicate semantic keys from distinct physical files are reported as conflicts;
- conflict entries do not cause prior semantic backup entries to be removed;
- backup preview shows both physical source and semantic key;
- redirects still work for legacy physical paths.

Add restore tests:

- semantic Windows backup restores to the current Windows user path;
- semantic Windows backup restores into a selected Wine prefix;
- semantic Windows backup restores through a relocated Windows KnownFolder;
- semantic `WinDrive('d')` restores through a matching Wine `dosdevices/d:`;
- missing drive mappings fail with an actionable message;
- semantic restore applies restore redirects after physical materialization;
- semantic keys, not physical paths, own game file selection when available;
- restore fails clearly when multiple prefixes are possible;
- legacy absolute backup continues to restore as before.

Add backup format tests:

- new semantic backups write `pathFormat: semantic-v1`;
- semantic backups reserve `registryFormat`;
- old backups without `pathFormat` remain readable;
- simple and zip formats use the same semantic storage path derivation;
- first semantic backup after a legacy chain is a full backup;
- differential backups compare semantic keys rather than physical Wine paths;
- semantic and legacy storage namespaces do not mix in one backup chain.

Add property-based or table-driven round-trip tests:

- `semantic(materialize(semantic, target), target) == semantic` for every
  supported semantic base;
- materializing a semantic key twice for the same target is stable;
- scanning the same physical path twice produces the same semantic key;
- consecutive semantic backups with unchanged files produce no new differential
  changes;
- changing a Wine prefix host path does not change the semantic key.

Add performance checks:

- semantic origin tracking and reverse mapping should not meaningfully regress
  large scans;
- establish a benchmark corpus with many games, roots, prefixes, and files;
- use that benchmark to set a concrete threshold before implementation is
  merged, for example a maximum percentage increase in scan time compared with
  the same corpus without semantic path derivation.

## Migration and compatibility

Existing backups must remain valid. Absence of the semantic path marker means
legacy absolute-path behavior.

New semantic backups should not rewrite existing legacy backups unless the user
creates a new backup. The initial implementation does not include a write
migration tool; preview/dry-run analysis is the required migration aid.

The first semantic backup for a game after a legacy backup must be full. Later
differential backups may be semantic only if their parent full backup is also
semantic.

During mixed history, Ludusavi may see both legacy physical keys and new
semantic keys. It should display them distinctly and avoid treating one as a
deletion of the other unless semantic equivalence is proven.

Manifest updates may change future semantic derivation, but they must not
rewrite existing backup keys. If a new manifest version causes a current scan to
produce a different semantic key for the same physical file, Ludusavi should
show that as a visible semantic-key change rather than mutating older mappings.

## Documentation updates

Update `docs/help/backup-structure.md`:

- explain semantic paths versus physical paths;
- clarify that usernames and Wine prefix paths are not part of new portable
  Windows/Wine backup identity.

Update `docs/help/backup-validation.md`:

- validate `pathFormat` and `registryFormat` per backup chain;
- validate semantic storage namespace structure;
- report duplicate semantic keys and mixed legacy/semantic chain data.

Update `docs/help/transfer-between-operating-systems.md`:

- document Windows/Wine support level;
- clarify that general native cross-OS support remains limited by manifest
  relationship data;
- mention the cloud-sync benefit of semantic storage paths when the same save is
  backed up from multiple machines.

Update `docs/help/roots.md`:

- explain that Wine prefix roots are used as scan/restore targets;
- clarify that the prefix itself is not stored as the save identity in semantic
  backups;
- clarify that Steam Proton compatdata prefixes do not need to be added as
  separate Wine prefix roots when a Steam root is configured; the Steam root acts
  as the prefix provider.

Update `docs/help/redirects.md`:

- distinguish redirects from semantic portability;
- note that redirects remain useful for custom layouts and unsupported cases.

Update schema and machine-readable output docs:

- `docs/schema/config.yaml` should document per-game preferred prefixes,
  optional Wine users, and optional drive mappings;
- `docs/schema/api-output.yaml` and `docs/schema/general-output.yaml` should
  expose semantic source keys separately from physical paths where preview or
  restore output reports them;
- `docs/schema/api-input.yaml` should document any new restore selection fields
  that accept semantic keys.

Update localization resources:

- add user-facing strings in `lang/*.ftl` for ambiguous prefix selection,
  invalid configured prefix, missing drive mapping, semantic-key conflict,
  foreign namespace, semantic mode preview, and switching to a new semantic full
  backup chain.

## Review checklist

Before opening a PR, verify these invariants:

- no new semantic path is accidentally interpreted as an OS path;
- a username change does not change the backup key for current-user Windows
  saves;
- a Wine prefix location change does not change the backup key;
- relocated Windows KnownFolders materialize through the current platform's
  location APIs;
- manifest-derived keys take precedence over platform reverse mapping;
- semantic keys have a single parser, serializer, and storage-path encoder;
- `<storeUserId>` remains part of Steam semantic keys;
- `<winHome>` is used only after more specific KnownFolder bases fail;
- overlapping manifest entries resolve by longest matched prefix, then
  declaration order;
- semantic key equality follows the semantic base case policy;
- physical redirects do not alter backup semantic keys;
- restore redirects run after semantic keys are materialized to physical paths;
- CLI `--wine-prefix` conflicts with per-game preferences fail visibly;
- Wine symlinks do not erase prefix context during semantic derivation;
- Steam userdata uses a store semantic namespace rather than a host root path;
- missing non-C drives fail visibly instead of being remapped to C;
- restoring to Wine requires one clear prefix;
- duplicate semantic keys from distinct physical files are reported as
  conflicts;
- semantic conflicts do not remove the prior backup entry for that key;
- switching from legacy to semantic keys starts a new full backup chain;
- first semantic backup preview explains the new full backup chain;
- physical ignores run before semantic derivation, while selection/toggles use
  semantic keys when present;
- schema docs distinguish semantic keys from physical paths in API output;
- manifest updates do not retroactively rewrite existing backup keys;
- ambiguous cases fail loudly instead of guessing;
- old backups are still readable and restorable;
- unsupported registry transfers are reported honestly.

## Implementation guide for coding agents

This section provides explicit, step-by-step instructions, acceptance criteria,
and progress tracking for each implementation unit. A coding agent should
complete each task in order, mark its status, and not proceed to the next task
until all acceptance criteria for the current task pass.

### Progress status markers

Use these markers at the start of each task heading:

- `[ ]` — not started
- `[~]` — in progress
- `[x]` — complete, all acceptance criteria pass
- `[!]` — blocked, with explanation

---

### Task 1: [x] Define `SemanticBase` enum and `SemanticPath` struct

**File(s):** Create `src/semantic.rs` (or a module under `src/path/`).

**What to implement:**

```rust
/// Represents a portable semantic location category.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SemanticBase {
    WinHome,
    WinDocuments,
    WinAppData,
    WinLocalAppData,
    WinLocalAppDataLow,
    WinSavedGames,
    WinPublic,
    WinProgramData,
    WinDir,
    WinDrive(char),
    SteamUserdata,
    // Future: XdgData, XdgConfig, Home, etc.
}

/// A portable save-file identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SemanticPath {
    pub base: SemanticBase,
    pub tail: String, // forward-slash separated, no leading slash
}
```

Add:
- `SemanticPath::parse(s: &str) -> Result<Self, SemanticPathError>` — parses
  `<baseName>/tail/path` format.
- `SemanticPath::serialize(&self) -> String` — canonical string form.
- `SemanticPath::storage_path(&self) -> String` — returns
  `__ludusavi_semantic__/<baseName>/tail`.
- `SemanticBase::case_sensitive(&self) -> bool` — returns `false` for all Win*
  bases, `true` for future Linux bases.
- `SemanticPath::eq_semantic(&self, other: &Self) -> bool` — equality respecting
  case policy.

**Acceptance criteria:**

1. `SemanticPath::parse("<winDocuments>/Game/save.dat")` succeeds and
   round-trips through `serialize()`.
2. `parse` rejects strings without a recognized `<base>` prefix.
3. `parse` rejects tails containing `.` or `..` components.
4. `parse` rejects empty tails.
5. `storage_path()` never contains `\`, always uses `/`.
6. `eq_semantic` is case-insensitive for `WinDocuments` base:
   `<winDocuments>/Game/Save.dat` == `<winDocuments>/game/save.dat`.
7. `eq_semantic` preserves case distinction for future case-sensitive bases.
8. All variants of `SemanticBase` serialize/deserialize through serde without
   data loss.
9. `WinDrive('d')` serializes as `<winDrive-d>` and parses back.
10. Unit tests cover all of the above.

---

### Task 2: [x] Implement physical-to-semantic conversion (Windows)

**File(s):** `src/semantic.rs` or `src/semantic/convert.rs`.

**What to implement:**

```rust
/// Convert a physical Windows path to a semantic path for the current user.
/// Returns None if the path cannot be semantically classified.
pub fn windows_physical_to_semantic(
    physical: &StrictPath,
    known_folders: &KnownFolders, // existing CommonPath/KnownFolder data
) -> Option<SemanticPath>
```

**Algorithm (must follow this exact priority order):**

1. Check `known_folders.saved_games` → `WinSavedGames`
2. Check `known_folders.documents` → `WinDocuments`
3. Check `known_folders.local_app_data` + `/Low` suffix → `WinLocalAppDataLow`
4. Check `known_folders.local_app_data` → `WinLocalAppData`
5. Check `known_folders.app_data` (Roaming) → `WinAppData`
6. Check `known_folders.public` → `WinPublic`
7. Check `known_folders.program_data` → `WinProgramData`
8. Check `known_folders.windows` → `WinDir`
9. Check `known_folders.user_profile` → `WinHome`
10. Extract drive letter, use `WinDrive(letter)` with path after `X:/`

Each check: if `physical` starts with the folder path (case-insensitive), strip
that prefix to get the tail.

**Acceptance criteria:**

1. `C:/Users/Alice/Documents/Game/save.dat` → `<winDocuments>/Game/save.dat`
   when Documents = `C:/Users/Alice/Documents`.
2. `C:/Users/Alice/AppData/Local/Game/save.dat` → `<winLocalAppData>/Game/save.dat`.
3. `C:/Users/Alice/AppData/Local/Low/Game/save.dat` → `<winLocalAppDataLow>/Game/save.dat`
   (LocalLow checked before Local).
4. `C:/Users/Alice/Saved Games/Game/save.dat` → `<winSavedGames>/Game/save.dat`.
5. Relocated Documents (e.g., `D:/MyDocs`) still works when `known_folders`
   reports the relocated path.
6. `D:/Games/save.dat` → `<winDrive-d>/Games/save.dat` when no KnownFolder
   matches.
7. `C:/Users/Alice/MyGames/save.dat` → `<winHome>/MyGames/save.dat` (not
   `<winDocuments>`).
8. Path comparison is case-insensitive on Windows.
9. Returns `None` for UNC paths or paths that cannot be parsed.
10. Unit tests cover all of the above, including edge cases with trailing
    slashes and mixed separators.

---

### Task 3: [x] Implement physical-to-semantic conversion (Wine prefix)

**File(s):** `src/semantic.rs` or `src/semantic/convert.rs`.

**What to implement:**

```rust
/// Convert a physical path inside a validated Wine prefix to a semantic path.
/// `prefix_path` is the validated prefix root (parent of drive_c).
/// `wine_user` is the detected Wine username for this prefix.
pub fn wine_physical_to_semantic(
    physical: &StrictPath,
    prefix_path: &StrictPath,
    wine_user: &str,
) -> Option<SemanticPath>
```

**Algorithm:**

1. Strip `prefix_path` from `physical` to get prefix-relative path.
2. Determine if path is under `drive_c/users/<wine_user>/` (case-insensitive).
3. If yes, extract the sub-path after the user directory and apply the same
   priority order as Task 2 but using Wine directory names:
   - `Saved Games` → `WinSavedGames`
   - `Documents` or `My Documents` → `WinDocuments`
   - `AppData/Local/Low` or `Local Settings/Application Data/Low` → `WinLocalAppDataLow`
   - `AppData/Local` or `Local Settings/Application Data` → `WinLocalAppData`
   - `AppData/Roaming` or `Application Data` → `WinAppData`
   - After all specific folders: remainder under user dir → `WinHome`
4. If path is under `drive_c/users/Public` → `WinPublic`
5. If path is under `drive_c/ProgramData` → `WinProgramData`
6. If path is under `drive_c/windows` → `WinDir`
7. If path is under `drive_<letter>` (not `drive_c`) → `WinDrive(letter)`
8. If path is under `drive_c` but not matched above → `WinDrive('c')` with
   path after `drive_c/`
9. Otherwise return `None`.

**Critical rules:**
- Use the **lexical** prefix-relative path, NOT `realpath`. Do not resolve
  symlinks before classification.
- All directory name comparisons are case-insensitive.
- The `wine_user` parameter comes from the caller (Task 5 handles detection).

**Acceptance criteria:**

1. `.../drive_c/users/steamuser/Documents/Game/save.dat` →
   `<winDocuments>/Game/save.dat`.
2. `.../drive_c/users/deck/AppData/Roaming/Game/save.dat` →
   `<winAppData>/Game/save.dat`.
3. `.../drive_c/users/steamuser/Application Data/Game/save.dat` →
   `<winAppData>/Game/save.dat` (XP alias).
4. `.../drive_c/users/steamuser/Local Settings/Application Data/Game/save.dat`
   → `<winLocalAppData>/Game/save.dat` (XP alias).
5. `.../drive_c/users/steamuser/My Documents/Game/save.dat` →
   `<winDocuments>/Game/save.dat` (XP alias).
6. `.../drive_c/ProgramData/Game/save.dat` → `<winProgramData>/Game/save.dat`.
7. `.../drive_d/Games/save.dat` → `<winDrive-d>/Games/save.dat`.
8. Symlinked Documents directory does NOT cause the path to escape prefix
   context (lexical path used).
9. Case variations like `appdata/roaming` still match.
10. Unit tests cover all of the above.

---

### Task 4: [x] Implement prefix validation

**File(s):** `src/semantic/prefix.rs` or similar.

**What to implement:**

```rust
pub struct ValidatedPrefix {
    pub path: StrictPath,
    pub wine_user: String,
    pub has_drive_c: bool,
    pub drive_mappings: HashMap<char, StrictPath>, // from dosdevices
}

/// Validate a candidate prefix path.
/// Returns None if validation fails.
pub fn validate_prefix(candidate: &StrictPath) -> Option<ValidatedPrefix>
```

**Validation rules:**
1. `candidate/drive_c` must exist as a directory.
2. At least one of `candidate/system.reg`, `candidate/user.reg`, or
   `candidate/dosdevices` must exist.
3. `candidate/drive_c/users` must exist as a directory.
4. Detect wine user: list entries in `drive_c/users`, exclude (case-insensitive)
   `Public`, `Default`, `Default User`, `All Users`. If exactly one remains,
   that is the wine user. If multiple remain, return the list for caller to
   resolve (or pick `steamuser` if present for Proton).
5. Scan `dosdevices/` for symlinks named `<letter>:` to build drive mappings.

**Acceptance criteria:**

1. A directory with `drive_c/` + `system.reg` passes.
2. A directory with `drive_c/` + `dosdevices/` passes.
3. A directory with only `drive_c/` (no reg files, no dosdevices) fails.
4. A directory without `drive_c/` fails.
5. Wine user detection excludes `Public`, `Default`, `Default User`, `All Users`
   (case-insensitive).
6. Single remaining user is returned as `wine_user`.
7. `dosdevices/d:` symlink pointing to `/mnt/data` produces
   `drive_mappings['d'] = /mnt/data`.
8. Invalid prefix logs a warning and returns `None`.
9. Unit tests with temp directories cover all cases.

---

### Task 5: [x] Implement Wine user detection for restore

**File(s):** Same module as Task 4.

**What to implement:**

```rust
/// Choose the Wine user for restore into a validated prefix.
pub fn choose_wine_user_for_restore(
    prefix: &ValidatedPrefix,
    game_config: Option<&GamePrefixConfig>, // per-game preferred user
    target_path_hint: Option<&str>,         // existing path to check
    is_proton: bool,
) -> Result<String, WineUserAmbiguity>
```

**Priority order:**
1. `game_config.preferred_wine_user` if set → return it.
2. If `target_path_hint` is provided and a user directory contains that path →
   return that user.
3. If only one non-system user exists → return it.
4. If `is_proton` and `steamuser` exists → return `steamuser`.
5. Otherwise → return `Err(WineUserAmbiguity { candidates })`.

**Acceptance criteria:**

1. Configured preferred user is always returned first.
2. Single-user prefix always succeeds without config.
3. Proton prefix with `steamuser` + `deck` returns `steamuser`.
4. Multi-user non-Proton prefix without config returns error with candidate list.
5. Unit tests cover all priority levels.

---

### Task 6: [x] Implement semantic-to-physical materialization

**File(s):** `src/semantic/materialize.rs` or similar.

**What to implement:**

```rust
/// Materialize a semantic path to a physical path on the current platform.
pub fn materialize_semantic(
    semantic: &SemanticPath,
    target: &MaterializeTarget,
) -> Result<StrictPath, MaterializeError>

pub enum MaterializeTarget {
    CurrentWindows { known_folders: KnownFolders },
    WinePrefix { prefix: ValidatedPrefix, wine_user: String },
}
```

**Rules:**
- `WinDocuments` + `CurrentWindows` → `known_folders.documents / tail`
- `WinDocuments` + `WinePrefix` → `prefix.path / drive_c / users / wine_user / Documents / tail`
- `WinDrive('d')` + `WinePrefix` → use `prefix.drive_mappings['d']` or
  `prefix.path / drive_d / tail`; error if neither exists.
- `WinDrive('d')` + `CurrentWindows` → `D:/ tail`; error if drive doesn't exist.
- All Win* bases follow the same pattern.
- Verify final path length is within long-path limits or return error.

**Acceptance criteria:**

1. `<winDocuments>/Game/save.dat` + Windows target → uses actual KnownFolder
   Documents path.
2. `<winDocuments>/Game/save.dat` + Wine target → `prefix/drive_c/users/steamuser/Documents/Game/save.dat`.
3. `<winDrive-d>/Games/save.dat` + Wine target with `dosdevices/d:` →
   resolves through the mapping.
4. `<winDrive-d>/Games/save.dat` + Wine target without d: mapping → error.
5. `<winDrive-d>/Games/save.dat` + Windows target without D: drive → error.
6. Round-trip: `wine_physical_to_semantic` then `materialize_semantic` back to
   Wine produces the original path (modulo case normalization).
7. Round-trip: `windows_physical_to_semantic` then `materialize_semantic` back
   to Windows produces the original path.
8. Long path (>260 chars) on Windows without long-path support → error.
9. Unit tests cover all bases × both targets.

---

### Task 7: [x] Implement manifest-origin tracking in scan

**File(s):** Modify `src/scan.rs` and related scan infrastructure.

**What to implement:**

Add an `origin` field to scan results that records:
- which manifest entry matched;
- which placeholder was expanded;
- which root/store provided the match;
- the matched prefix length and remaining tail.

This metadata is used by Task 8 to derive manifest-based semantic keys.

**Acceptance criteria:**

1. After scanning a game, each `ScannedFile` carries origin metadata when it
   came from a manifest entry.
2. Origin includes the manifest placeholder string (e.g., `<winDocuments>/Remedy/Alan Wake`).
3. Origin includes the root kind (Steam, Heroic, Lutris, Other, WinePrefix).
4. Origin includes the expanded prefix that was stripped to find the tail.
5. Files found through custom game entries or non-manifest sources have
   `origin = None`.
6. Existing scan behavior and results are unchanged (no regressions in
   `cargo test`).
7. Integration test: scan a game with a known manifest entry and verify origin
   is populated.

---

### Task 8: [x] Implement manifest-derived semantic key generation

**File(s):** `src/semantic/derive.rs` or similar.

**What to implement:**

```rust
/// Derive a semantic key from manifest origin metadata.
/// Returns None if the origin does not support semantic derivation.
pub fn derive_from_manifest_origin(
    origin: &ScanOrigin,
    physical: &StrictPath,
) -> Option<SemanticPath>
```

**Rules:**
- If the manifest placeholder is a recognized semantic base (e.g.,
  `<winDocuments>`, `<winAppData>`), use it directly with the file tail.
- If the manifest uses `<root>/userdata/<storeUserId>/<id>` and root is Steam,
  produce `<steamUserdata>/<storeUserId>/<id>/tail`.
- If the manifest uses `<base>` or `<root>` with a non-portable root, return
  `None` (fall through to reverse mapping).
- When multiple manifest entries match, choose longest matched prefix, then
  declaration order.

**Source precedence enforcement:**
- This function is called FIRST. Only if it returns `None` should the caller
  invoke `windows_physical_to_semantic` or `wine_physical_to_semantic`.

**Acceptance criteria:**

1. Manifest entry `<winDocuments>/Remedy/Alan Wake` + file tail `save.dat` →
   `<winDocuments>/Remedy/Alan Wake/save.dat`.
2. Steam userdata manifest entry → `<steamUserdata>/12345/67890/remote/save.dat`.
3. Generic `<base>/saves` with non-Steam root → returns `None`.
4. Two overlapping entries: longer match wins.
5. Same-length entries: first in manifest wins.
6. `<storeUserId>` is preserved in the key (different accounts = different keys).
7. Unit tests cover all cases.

---

### Task 9: [x] Integrate semantic keys into backup planning

**File(s):** Modify `src/scan.rs`, `src/backup.rs`, `src/layout.rs`.

**What to implement:**

- After scan, for each `ScannedFile`:
  1. Try `derive_from_manifest_origin` (Task 8).
  2. If None and file is in a validated Wine prefix, try
     `wine_physical_to_semantic` (Task 3).
  3. If None and platform is Windows, try `windows_physical_to_semantic`
     (Task 2).
  4. If None, keep legacy physical-path behavior.
- Store the semantic key (when available) as the `mapping_key` in backup
  planning.
- Write `pathFormat: semantic-v1` in `mapping.yaml` for new backups that use
  semantic keys.
- Write `registryFormat: unsupported` in `mapping.yaml`.
- Use `__ludusavi_semantic__/` storage path prefix for semantic entries.
- Detect format switch: if previous full backup is legacy and new scan produces
  semantic keys, force a new full backup.

**Acceptance criteria:**

1. A Wine prefix scan produces semantic keys in `mapping.yaml`.
2. A Windows scan produces semantic keys in `mapping.yaml`.
3. `pathFormat: semantic-v1` appears in new backup metadata.
4. `registryFormat: unsupported` appears in new backup metadata.
5. Storage files land under `__ludusavi_semantic__/winDocuments/...` etc.
6. Legacy backups without `pathFormat` still load and restore correctly.
7. First semantic backup after legacy chain is a full backup (not differential).
8. Differential backup within a semantic chain compares semantic keys correctly.
9. No `drive-*` folders appear in a semantic backup chain.
10. `cargo test` passes with no regressions.

---

### Task 10: [x] Integrate semantic keys into restore planning

**File(s):** Modify `src/restore.rs` and related.

**What to implement:**

- When reading a backup with `pathFormat: semantic-v1`:
  1. Parse each file key as `SemanticPath`.
  2. Determine `MaterializeTarget` (Windows or Wine prefix).
  3. Call `materialize_semantic` to get physical restore path.
  4. Apply restore redirects to the materialized physical path.
  5. Use the final path for file writing.
- When no prefix is available for Wine semantic keys on Linux, return
  actionable error.
- Preferred prefix resolution uses the priority from the plan.

**Acceptance criteria:**

1. Semantic backup created on Linux/Wine restores correctly on Windows.
2. Semantic backup created on Windows restores correctly into a Wine prefix.
3. Restore preview shows semantic source + physical target.
4. Missing prefix → clear error message, not silent failure.
5. Ambiguous prefix → clear error with candidate list.
6. Restore redirects apply AFTER materialization.
7. Legacy backups restore unchanged.
8. `cargo test` passes.

---

### Task 11: [x] Implement duplicate semantic key conflict detection

**File(s):** Modify scan/backup planning.

**What to implement:**

- During scan, if two distinct physical files produce the same semantic key
  (via `eq_semantic`):
  - Do NOT choose one silently.
  - Mark both as conflicted.
  - In preview, show the conflict with both physical sources.
  - Do NOT remove the prior backup entry for that key.
  - Block backup for that specific key until user resolves (disable one root,
    or configure more specific root).

**Acceptance criteria:**

1. Two files from different roots with same semantic key → conflict reported.
2. Same file via two aliases (symlink) → collapsed, no conflict.
3. Conflict does not delete existing backup entry.
4. Preview clearly shows which files conflict and suggests resolution.
5. Non-conflicting files in the same game still back up normally.
6. Unit test with mock scan data.

---

### Task 12: [x] Implement Phase 4 warning signals

**File(s):** New module or extend backup comparison logic.

**What to implement:**

Four signal types when comparing current scan to existing backup:
- `sameSemanticKey` — same key exists in both.
- `sameNamespaceMissing` — backup has keys in a namespace the current platform
  understands, but current scan has no match.
- `foreignNamespace` — backup has keys in a namespace the current platform
  cannot materialize.
- `ambiguousMaterialization` — key is understood but multiple physical targets
  exist.

**Acceptance criteria:**

1. Windows scan sees Wine-created `<winDocuments>` backup → `sameSemanticKey`.
2. Windows scan has no `<winDrive-d>` match but backup does → `sameNamespaceMissing`.
3. Linux scan without prefix sees `<winDocuments>` backup → `foreignNamespace`
   (no prefix configured).
4. Linux scan with two valid prefixes → `ambiguousMaterialization`.
5. `foreignNamespace` entries are NOT removed from backup.
6. `ambiguousMaterialization` blocks restore for that key.
7. Unit tests for each signal.

---

### Task 13: [x] Implement preview/dry-run analysis (Phase 4.5)

**File(s):** Extend CLI `--preview` output.

**What to implement:**

When running backup preview, additionally show:
- Which legacy keys would become semantic keys (and what they'd be).
- Which games would start a new full backup chain.
- Which configured prefixes failed validation (and why).
- Which semantic keys conflict.
- One-time notice about format switch.

**Acceptance criteria:**

1. Preview output includes "would switch to semantic-v1" notice per game.
2. Preview shows old physical key → new semantic key mapping.
3. Invalid prefix paths are reported with reason.
4. Conflicts are shown with both physical sources.
5. No actual backup data is written during preview.
6. Integration test with a mixed legacy/new scenario.

---

### Task 14: [x] Update documentation and localization

**File(s):** `docs/help/*.md`, `docs/schema/*.yaml`, `lang/*.ftl`.

**What to implement:**

Per the "Documentation updates" section of this plan:
- Update all listed help documents.
- Add schema fields for preferred prefix config and semantic key output.
- Add `lang/en-US.ftl` keys for all new user-facing messages:
  - `semantic-prefix-ambiguous`
  - `semantic-prefix-invalid`
  - `semantic-drive-missing`
  - `semantic-key-conflict`
  - `semantic-foreign-namespace`
  - `semantic-format-switch-notice`
  - `semantic-preview-would-become`

**Acceptance criteria:**

1. All listed docs are updated with accurate information.
2. Schema files validate against the new config/output structure.
3. `en-US.ftl` has all new keys.
4. Other `lang/*.ftl` files have the same keys with English fallback (or
   marked for translation).
5. No broken links in documentation.

---

### Task 15: [x] Property-based and round-trip tests

**File(s):** `tests/semantic_properties.rs` or similar.

**What to implement:**

Using `proptest` or `quickcheck`:
1. For every `SemanticBase` variant, generate random tails and verify:
   - `parse(serialize(path)) == path`
   - `materialize` then re-derive produces the same semantic key
2. Consecutive backups with unchanged files produce zero differential entries.
3. Changing Wine prefix host path does not change semantic key.
4. Changing Windows username does not change semantic key.
5. `storage_path` never contains OS-specific separators.

**Acceptance criteria:**

1. Property tests run in CI (`cargo test`).
2. At least 1000 iterations per property.
3. All properties pass.
4. Shrunk failure cases are human-readable.

---

### Task 16: [x] Performance benchmark

**File(s):** `benches/semantic_scan.rs` or similar.

**What to implement:**

- Create a benchmark corpus with 500+ games, multiple roots, multiple prefixes.
- Measure scan time with and without semantic derivation.
- Set threshold: semantic derivation must not increase scan time by more than
  15% compared to legacy-only scan on the same corpus.

**Acceptance criteria:**

1. Benchmark exists and runs via `cargo bench`.
2. Results are documented in PR.
3. Threshold is met on CI hardware.

---

## Cross-task invariants (verify after ALL tasks complete)

These are global properties that must hold across the entire implementation.
Run these checks as a final validation pass:

1. `cargo test` passes with zero failures.
2. `cargo clippy` produces no warnings.
3. No `unsafe` code introduced.
4. No panics (`unwrap`/`expect`) on user-controlled input paths.
5. All new public APIs have doc comments.
6. Semantic key strings never appear in filesystem API calls without going
   through `materialize_semantic` first.
7. The word `semantic` does not appear in any user-facing GUI text (use
   "portable" or "cross-platform" instead).
8. Backup/restore of 10+ real games works end-to-end in a manual smoke test
   covering: Windows native, Wine/Proton on Linux, legacy backup compatibility.
9. `mapping.yaml` files written by the new code can be read by the previous
   release (graceful unknown-field handling).
10. Previous-release `mapping.yaml` files are read correctly by the new code.

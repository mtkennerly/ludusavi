# Backup structure
* Within the target folder, for every game with data to back up, a subfolder
  will be created based on the game's name, where some invalid characters are
  replaced by `_`. In rare cases, if the whole name is invalid characters,
  then it will be renamed to `ludusavi-renamed-<ENCODED_NAME>`.
* Within each game's subfolder, there will be a `mapping.yaml` file that
  Ludusavi needs to identify the game.

  When using the simple backup format, there will be some drive folders
  (e.g., `drive-C` on Windows or `drive-0` on Linux and Mac) containing the
  backup files, matching the normal file locations on your computer.
  When using the zip backup format, there will be zip files instead.
* If the game has save data in the registry and you are using Windows, then
  the game's subfolder will also contain a `registry.yaml` file (or it will
  be placed in each backup's zip file).
  If you are using Steam and Proton instead of Windows, then the Proton `*.reg`
  files will be backed up along with the other game files instead.

During a restore, Ludusavi only considers folders with a `mapping.yaml` file.

## Absolute vs relative paths
A common question is why Ludusavi stores files by their absolute path (e.g., `C:\Users\foo\save.txt`),
rather than using relative paths or placeholders (e.g., `%USERPROFILE%\save.dat`).
The motivation for this question is usually because people use multiple systems,
and the backed up files are tied to different usernames on each system
(or different Steam library locations, etc).
Although you can typically solve this by [configuring redirects](/docs/help/redirects.md),
people often wonder why Ludusavi doesn't do that automatically.

The reason is that it may work in simple cases, but not in complex or unusual ones,
so Ludusavi errs on the side of caution.
For example, consider these potential challenges:

* You can configure Ludusavi to back up data from two different users on the same system at the same time.
  It could become ambiguous which user folder to use when restoring the backup.
* On Windows, you can relocate special system folders,
  or on Linux, you can change the `XDG` variables,
  but some games may not respect that.
  Let's say Ludusavi backed up `C:\Users\foo\Documents\Some Game`,
  and then you used Windows Explorer -> Properties -> Location to move the Documents folder somewhere else.
  Ludusavi wouldn't know if that game would use the Windows API to find and honor the new Documents location,
  or if it would ignore that and use the standard location.
* You can have multiple Steam libraries on the same system,
  or the same game may belong to the primary library on one system and the secondary library on another.

Using absolute paths is the safest way to ensure that backups are always restored to the same place on the same system.
The trade-off is that you must define redirects to help Ludusavi understand your unique setup.

## Semantic paths (cross-platform backups)

For Windows and Wine/Proton saves, Ludusavi supports a portable path format
called **semantic paths**. Instead of storing the literal file path
(e.g., `/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat`),
the backup stores a portable identity like `<winDocuments>/Game/save.dat`.

This format:

- Ignores Windows usernames (`Alice`, `Bob`) and Wine usernames (`steamuser`, `deck`)
- Ignores Wine prefix locations (`/home/deck/Prefixes/Game`)
- Preserves the semantic meaning of the save location (Documents, AppData, etc.)
- Enables cloud-sync deduplication across machines

When restoring a semantic backup:

- On Windows: files go to the current user's actual Documents/AppData/etc. folders
- In Wine: files go to the selected prefix's `drive_c/users/<wine-user>/Documents` etc.

Semantic backups use `pathFormat: semantic-v1` in `mapping.yaml`
and store files under a `__ludusavi_semantic__/` namespace
within the backup folder.

### Multi-prefix context metadata

When a game's saves come from multiple Wine prefixes (e.g., two different Heroic
installations), the backup stores **context metadata** to track which prefix each
file originated from. This ensures that:

- Files from different prefixes don't collide when they share the same semantic key
- On restore, each file returns to its correct prefix

**Mapping key format:**

| Type | Format | Example |
|------|--------|---------|
| No context | `<baseName>/<tail>` | `<winDocuments>/Game/save.dat` |
| With context | `__ludusavi_context__/<N>/<baseName>/<tail>` | `__ludusavi_context__/0/<winDocuments>/Game/save.dat` |
| Legacy absolute | Anything else | `C:\Users\me\Documents\Game\save.dat` |

**Storage path format:**

| Type | Storage path |
|------|-------------|
| No context | `__ludusavi_semantic__/<baseName>/<tail>` |
| With context | `__ludusavi_context__/<N>/__ludusavi_semantic__/<baseName>/<tail>` |

Context IDs (`<N>`) are assigned deterministically by sorting prefixes
canonically by `(prefix_path, wine_user)`. Single-prefix backups use plain
semantic keys (no context prefix) but still store the source prefix info in
`mapping.yaml` under `pathContexts` for future cross-machine restore.

The `pathContexts` field in `mapping.yaml` maps context IDs to prefix metadata:

```yaml
pathContexts:
  0:
    prefixPath: /home/deck/Prefixes/Game1
    wineUser: steamuser
    driveMappings:
      z: /home/deck/Prefixes/Game1/dosdevices/z
  1:
    prefixPath: /home/user/.wine
    wineUser: user
```

Only `FullBackup` entries store `pathContexts`. `DifferentialBackup` entries
reference context IDs defined in their parent full backup. If a scan's
non-empty context metadata differs from the parent full backup, Ludusavi
forces a new full backup.

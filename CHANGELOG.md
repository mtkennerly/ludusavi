## Unreleased

* Added:
  * An indication when a single file or registry key will be backed up by
    more than one game.
  * CLI: A `complete` command for generating shell completion scripts.
  * GUI: Info about how many games are selected, if the number is different
    than how many games have been processed in the backup/restore.
  * GUI: An edit button next to each game in backup mode to quickly create
    a custom entry for that game, pre-filled with the default info.
  * GUI: A search option to just see games with matching names.
* Changed:
  * GUI: The backup/restore confirmation screen now has some extra text that
    recommends doing a preview first.
* Fixed:
  * When using a custom entry to override a known game, some of the original's
    non-overridable data would not be inherited by the custom entry (namely its
    Steam ID and installation directory name).
  * GUI: If the initial full scan found saves for a game, and then you created
    a custom entry for that game such that no saves would be found, did a scan,
    deleted the custom entry, and did another scan, then that scan would not
    find any files for the game. Now, it will correctly revert to the standard
    data for that game immediately.

## v0.8.0 (2020-08-10)

* Added:
  * If you create a file called `ludusavi.portable` in the same location as
    the executable, then Ludusavi will store its config file and the manifest
    there as well.
* Fixed:
  * Read-only files could only be backed up once, since the original backup
    could not be replaced by a newer copy, and you could not restore a backup
    if the original file was read-only. Now, Ludusavi will try to unset the
    read-only flag on backups before replacing them with newer backups, and
    it will try to unset the flag on target files before restoring a backup.
  * Invalid paths like `C:\Users\Foo\Documents\C:\Users\Foo` would be shortened
    to just `C:\Users\Foo`, which could cause irrelevant files to be backed up.
    Now, the extraneous `C:` will be converted to `C_` so that it simply won't
    match any files or directories.
  * When some games were deselected, the disk space display only showed units
    for the total space, not the used space, which could lead to it showing
    "1.42 of 1.56 GiB", where 1.42 was actually MiB and not GiB.
    Units are now shown for both sides.
* Changed:
  * When backing up or restoring a file, if it already exists with the correct
    content, then Ludusavi won't re-copy it.
  * In GUI mode, Ludusavi now tries to be smarter about when a full scan is
    needed. Previously, every backup and backup preview would trigger a full
    scan. Now, Ludusavi will remember which games it found and only re-scan
    those games (until you change your roots, change the "other" settings,
    or reopen the program).
  * In CLI mode, `--try-update` will use a default, empty manifest if there is
    no local copy of the manifest and it cannot be downloaded.

## v0.7.0 (2020-08-01)

**The backup structure has changed! Read below for more detail.**

* Added:
  * Backup option to exclude save locations that are only confirmed for
    another operating system.
  * Backup option to exclude store screenshots.
  * `--try-update` flag for backups via CLI.
* Fixed:
  * When starting the GUI, if Ludusavi could not check for a manifest update
    (e.g., because your Internet is down), then it would default to an empty
    manifest even if you already had a local copy that was downloaded before.
    Now, it will use the local copy even if it can't check for updates.
* Changed:
  * Backup structure is now human-readable.
  * App window now has a minimum size, 640x480.
    (Note: For now, the crates.io release will not have a minimum size.)
  * File size units are now adjusted based on the size, rather than always using MiB.
    ([contributed by wtjones](https://github.com/mtkennerly/ludusavi/pull/32))

### New backup structure
Previously, Ludusavi used Base64 to encode game names and original paths when
organizing backups. There were some technical advantages of that approach, but
it was not easy to understand, and there was a technical flaw because Base64
output can include `/`, which isn't safe for folder or file names.

Therefore, Ludusavi now organizes backups like this, in a way that is easier
to read and understand:

```
C:/somewhere/the-backup-folder/
  Game 1 Name/
    mapping.yaml
    registry.yaml
    drive-C/  # drive-0 on Linux and Mac
      Users/
        ...
      Program Files/
        Steam/
          ...
```

The name of each game's folder is as close to the real title as possible,
except for replacing some special characters with `_`. Ultimately, Ludusavi
doesn't care much about the folder name and mainly looks for `mapping.yaml`,
which contains some metadata that Ludusavi needs. If a game has any Windows
registry data to back up, then there will also be a `registry.yaml` file.
Within each drive folder, everything is simply organized exactly like it
already is on your computer.

If you need to restore a previous backup, then please use Ludusavi v0.6.0
to do the restoration first, then migrate to Ludusavi v0.7.0 and create a
new backup.

You can [read more here](https://github.com/mtkennerly/ludusavi/issues/29)
about the background of this change. Be assured that this sort of disruptive
change is not taken lightly, but may happen in some cases until Ludusavi
reaches version 1.0.0.

## v0.6.0 (2020-07-29)

* Added:
  * Option to merge into an existing backup directory.
  * `--api` flag in CLI mode.
  * `--by-steam-id` flag in CLI mode.
* Fixed:
  * Registry values of type `EXPAND_SZ` and `MULTI_SZ` were converted to `SZ` when restored.
* Changed:
  * On Windows, the program icon is now embedded in the executable so that
    you can see it in the file browser as well.

## v0.5.0 (2020-07-25)

* Added:
  * Support for custom games.
  * Icons for several buttons.
  * An icon for Ludusavi itself.
    (Note: For now, the crates.io release will not show this icon.)
  * Support for cutting in text fields.
    (Note: For now, the crates.io release will copy text instead of cutting it.)
  * More buttons for browsing folders.
  * Support for `~` (user home directory) in redirects.
  * Support for `.` and `..` path segments when the path does not exist.
* Fixed:
  * On Windows, long paths can now be backed up without issue.
  * When backing up files, the Base64-encoded name now preserves the original
    file's actual capitalization, rather than the expected capitalization
    from the manifest.
  * There was a rare issue related to the above point where some files could be
    backed up twice, once with the original capitalization and once with the
    expected capitalization.
  * The CLI required the backup `--path` to already exist.
  * Keyboard shortcuts didn't work in redirect fields.
  * Registry keys were not backed up if the parent key had no values.
  * CLI mode would panic when restoring if a non-Base64-encoded file were
    present in the source folder. Now, such files will be reported as an error.
* Changed:
  * The configuration auto-save is now more predictable. All config changes
    are now saved immediately.
  * When a game has registry data to back up, registry.yaml no longer includes
    unnecessary fields and is now sorted alphabetically. This means that identical
    registry content will produce an identical registry.yaml across backups.
  * The progress bar is now shown on all screens.

## v0.4.0 (2020-07-21)

* Added the ability to select and deselect specific games.
* Added the ability to restore to different folders via redirects.
* Added indicators for how much disk space is used by the files.
* Added indicators in the GUI when files fail to process.
* Added a browse button for folders.
* Replaced the "=> Restore" and "=> Backup" buttons with a navigation bar.
* Redesigned the confirmation and error screens so that the buttons are shown
  below the text, which helps to prevent any accidental clicks before reading.
* Narrowed how Steam IDs are substituted in paths to avoid false positives.
* Fixed an issue where restore mode in the GUI would get stuck showing an
  "in progress" state if the source path had no subdirectories.

## v0.3.0 (2020-07-12)

* Added command line interface.
* Added common roots for GOG Galaxy on Windows.
* Added copy/undo/redo shortcuts in text fields. Cutting is not yet supported
  because of some limitations in the GUI library.
* Changed scrollbar style so that it's more obvious what's scrollable.
* Changed build process to avoid potential "VCRUNTIME140_1.dll was not found"
  error on Windows.

## v0.2.0 (2020-07-06)

* Added core backup/restore functionality.
* Added support for saves in the Windows registry.
* Added support for Steam + Proton saves.
* Added support for Steam screenshots.

## v0.1.0 (2020-06-20)

* Initial release.
* Just a prototype/mock-up and not yet functional.

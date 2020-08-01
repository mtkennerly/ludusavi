## Unreleased

**The backup structure has changed! Read below for more detail.**

* Changed:
  * Backup structure is now human-readable.

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

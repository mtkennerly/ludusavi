## Unreleased

* Added support for custom games.
* Added an icon for Ludusavi.
  (Note: For now, the crates.io release will not show an icon.)
* Added support for cutting in text fields.
  (Note: For now, the crates.io release will copy text instead of cutting it.)
* Overhauled path handling:
  * On Windows, long paths can now be backed up without issue.
  * When backing up files, the Base64-encoded name now preserves the original
    file's actual capitalization, rather than the expected capitalization
    from the manifest.
  * Fixed a rare issue related to the above point where some files could be
    backed up twice, once with the original capitalization and once with the
    expected capitalization.
  * Fixed an issue where the CLI required the backup `--path` to already exist.
  * Added support for `~` (user home directory) in redirects.
  * Added support for `.` and `..` path segments when the path does not exist.
* Fixed an issue where keyboard shortcuts didn't work in redirect fields.
* Made the configuration auto-save more predictable. Now, all config changes
  are saved immediately.

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

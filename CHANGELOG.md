## Unreleased

* Added the ability to select and deselect specific games.
* Added the ability to restore to different folders via redirects.
* Added indicators for how much disk space is used by the files.
* Added indicators in the GUI when files fail to process.
* Added a browse button for folders.
* Replaced the "=> Restore" and "=> Backup" buttons with a navigation bar.
* Redesigned confirmation and error screens so that the buttons are shown below
  the text, which helps to prevent any accidental clicks before reading.
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

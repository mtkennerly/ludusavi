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

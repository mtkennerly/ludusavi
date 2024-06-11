## Backup structure
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

# ![Logo](assets/icon.svg) Ludusavi
Ludusavi is a tool for backing up your PC video game save data,
written in [Rust](https://www.rust-lang.org).
It is cross-platform and supports multiple game stores.

## Features
* Ability to back up data from more than 8,000 games plus your own custom entries.
* Backup and restore for Steam as well as other game libraries.
* Preview of the backup/restore before actually performing it.
* Both a graphical interface and command line interface for scripting.
  Tab completion is available for Bash, Fish, Zsh, PowerShell, and Elvish.
* Support for:
  * Saves that are stored as files and in the Windows registry.
  * Proton saves with Steam.
  * Steam screenshots.
* Available as a [Playnite](https://playnite.link) extension:
  https://github.com/mtkennerly/ludusavi-playnite

This tool uses the [Ludusavi Manifest](https://github.com/mtkennerly/ludusavi-manifest)
for info on what to back up, and it will automatically download the latest version of
the primary manifest. The data is ultimately sourced from [PCGamingWiki](https://www.pcgamingwiki.com/wiki/Home),
so please contribute any new or fixed data back to the wiki itself, and your
improvements will be incorporated into Ludusavi's data as well.

If you'd like to help translate Ludusavi into other languages,
[check out the Crowdin project](https://crowdin.com/project/ludusavi).

## Demo
### GUI
> ![GUI demo of previewing a backup](docs/demo-gui.gif)

### CLI
> ![CLI demo of previewing a backup](docs/demo-cli.gif)

## Installation
### Requirements
* Ludusavi is available for Windows, Linux, and Mac.
* Your system must support OpenGL.

### Methods
You can install Ludusavi one of these ways:

* Download the executable for your operating system from the
  [releases page](https://github.com/mtkennerly/ludusavi/releases).
  It's portable, so you can simply download it and put it anywhere
  on your system.
  **If you're unsure, choose this option.**

* On Windows, you can use [Scoop](https://scoop.sh). To install, run:

  ```
  scoop bucket add extras
  scoop install ludusavi
  ```

  To update, run:

  ```
  scoop update
  scoop update ludusavi
  ```

* If you have [Rust](https://www.rust-lang.org), you can use Cargo. To install or update, run:

  ```
  cargo install ludusavi
  ```

  On Linux, this requires the following system packages, or their equivalents
  for your distribution:

  * Ubuntu: `sudo apt-get install -y gcc cmake libx11-dev libxcb-composite0-dev libfreetype6-dev libexpat1-dev libfontconfig1-dev`

### Notes
If you are on Windows:

* When you first run Ludusavi, you may see a popup that says
  "Windows protected your PC", because Windows does not recognize the program's
  publisher. Click "more info" and then "run anyway" to start the program.

If you are on Mac:

* When you first run Ludusavi, you may see a popup that says
  "Ludusavi can't be opened because it is from an unidentified developer".
  To allow Ludusavi to run, please refer to [this article](https://support.apple.com/en-us/HT202491),
  specifically the section on `How to open an app [...] from an unidentified developer`.

## Usage
### GUI
#### Backup mode
<details>
<summary>Click to expand</summary>

* This is the default mode when you open the program.
* You can press `preview` to see what the backup will include,
  without actually performing it.

  After you've done one preview or backup, Ludusavi will remember which games
  it found and only re-scan those games the next time. If you change your root
  configuration, change the "other" settings, or reopen the program, then
  it will do another full scan.
* You can press `back up` to perform the backup for real.
  * If the target folder already exists, it will be deleted first and
    recreated, unless you've enabled the merge option.
  * Within the target folder, for every game with data to back up, a subfolder
    will be created based on the game's name, where some invalid characters are
    replaced by `_`. In rare cases, if the whole name is invalid characters,
    then it will be renamed to `ludusavi-renamed-<ENCODED_NAME>`.
  * Within each game's subfolder, there will be a `mapping.yaml` file that
    Ludusavi needs to identify the game. There will be some drive folders
    (e.g., `drive-C` on Windows or `drive-0` on Linux and Mac) containing the
    backup files, matching the normal file locations on your computer.
  * If the game has save data in the registry and you are using Windows, then
    the game's subfolder will also contain a `registry.yaml` file.
    If you are using Steam and Proton instead of Windows, then the Proton `*.reg`
    files will be backed up along with the other game files instead.
* Roots are folders that Ludusavi can check for additional game data. When you
  first run Ludusavi, it will try to find some common roots on your system, but
  you may end up without any configured. You can click `add root` to configure
  as many as you need, along with the root's type:
  * For a Steam root, this should be the folder containing the `steamapps` and
    `userdata` subdirectories. Here are some common/standard locations:
    * Windows: `C:/Program Files (x86)/Steam`
    * Linux: `~/.steam/steam`
  * For the "other" root type and the remaining store-specific roots,
    this should be a folder whose direct children are individual games.
    For example, in the Epic Games store, this would be what you choose as the
    "install location" for your games (e.g., if you choose `D:/Epic` and it
    creates a subfolder for `D:/Epic/Celeste`, then the root would be `D:/Epic`).
  * For a home folder root, you may specify any folder. Whenever Ludusavi
    normally checks your standard home folder (Windows: `%USERPROFILE%`,
    Linux/Mac: `~`), it will additionally check this root. This is useful if
    you set a custom `HOME` to manipulate the location of save data.
  * For a Wine prefix root, this should be the folder containing `drive_c`.
    Currently, Ludusavi does not back up registry-based saves from the prefix,
    but will back up any file-based saves.
* To select/deselect specific games, you can run a preview, then click the
  checkboxes by each game. You can also press the `deselect all` button
  (when all games are selected) or the `select all` button (when at least
  one game is deselected) to quickly toggle all of them at once.
  Ludusavi will remember your most recent checkbox settings.
* Next to each game's name is an edit icon. Clicking this will create a custom
  game entry with the same name, allowing you to override that game's data.
  See the [custom games](#custom-games) section for more information.

  There is also a globe icon, which will open the game's PCGamingWiki article
  so that you can quickly double check or update its information if needed.
* You can click the search icon and enter some text to just see games with
  matching names. Note that this only affects which games you see in the list,
  but Ludusavi will still back up the full set of games.
* You may see a "duplicates" badge next to some games. This means that some of
  the same files were also backed up for another game. That could be intentional
  (e.g., an HD remaster may reuse the original save locations), but it could
  also be a sign of an issue in the manifest data. You can expand the game's
  file list to see which exact entries are duplicated.

</details>

#### Restore mode
<details>
<summary>Click to expand</summary>

* Switch to restore mode by clicking the `restore mode` button.
* You can press `preview` to see what the restore will include,
  without actually performing it.
* You can press `restore` to perform the restore for real.
  * For each subfolder in the source directory, Ludusavi looks for a `mapping.yaml`
    file in order to identify each game. Subfolders without that file, or with an
    invalid one, are ignored.
  * All files from the drive folders are copied back to their original locations
    on the respective drive. Any necessary parent directories will be created
    as well before the copy, but if the directories already exist, then their
    current files will be left alone (other than overwriting the ones that are
    being restored from the backup).
  * If the game subfolder includes a `registry.yaml` file, then the Windows
    registry data will be restored as well.
* You can use redirects to restore to a different location than the original file.
  Click `add redirect`, and then enter both the old and new location. For example,
  if you backed up some saves from `C:/Games`, but then you moved it to `D:/Games`,
  then you would put `C:/Games` as the source and `D:/Games` as the target.

  Tip: As you're editing your redirects, try running a preview and expanding some
  games' file lists. This will show you in real time what effect your redirects
  will have when you perform the restore for real.
* You can select/deselect specific games in restore mode just like you can in
  backup mode. The checkbox settings are remembered separately for both modes.
* You can click the search icon and enter some text to just see games with
  matching names. Note that this only affects which games you see in the list,
  but Ludusavi will still restore the full set of games.

</details>

#### Custom games
<details>
<summary>Click to expand</summary>

* Switch to this mode by clicking the `custom games` button.
* You can click `add game` to add entries for as many games as you like.
  Within each game's entry, you can click the plus icons to add paths
  (files or directories) and registry keys.
  * For paths, you can click the browse button to quickly select a folder.
    The path can be a file too, but the browse button only lets you choose
    folders at this time. You can just type in the file name afterwards.
  * In addition to regular paths, you can also use
    [globs](https://en.wikipedia.org/wiki/Glob_(programming))
    (e.g., `C:/example/*.txt` selects all TXT files in that folder)
    and the placeholders defined in the
    [Ludusavi Manifest format](https://github.com/mtkennerly/ludusavi-manifest).
* Make sure to give the game entry a name. Entries without names are ignored,
  as are empty paths and empty registry keys.

  If the game name matches one from Ludusavi's primary data set, then your
  custom entry will override it. This can be used to totally ignore a game
  (just don't specify any paths or registry) or to customize what is included
  in the backup.

</details>

#### Other settings
* Switch to this screen by clicking the `other` button.
* This screen contains some additional settings that are less commonly used.

### CLI
Run `ludusavi --help` for the full usage information.

#### API output
<details>
<summary>Click to expand</summary>

CLI mode defaults to a human-readable format, but you can switch to a
machine-readable JSON format with the `--api` flag. In that case, the output
will have the following structure:

* `errors` (optional, map):
  * `someGamesFailed` (optional, boolean): Whether any games failed.
  * `unknownGames` (optional, list of strings): Names of unknown games, if any.
* `overall` (map):
  * `totalGames` (number): How many games were found.
  * `totalBytes` (number): How many bytes are used by files associated with
    found games.
  * `processedGames` (number): How many games were processed.
    This excludes ignored, failed, and cancelled games.
  * `processedBytes` (number): How many bytes were processed.
    This excludes ignored, failed, and cancelled games.
* `games` (map):
  * Each key is the name of a game, and the value is a map with these fields:
    * `decision` (string): How Ludusavi decided to handle this game.

      Possible values:
      * `Processed`
      * `Ignored`
      * `Cancelled`
    * `files` (map):
      * Each key is a file path, and each value is a map with these fields:
        * `failed` (optional, boolean): Whether this entry failed to process.
        * `bytes` (number): Size of the file.
        * `originalPath` (optional, string): If the file was restored to a
          redirected location, then this is its original path.
        * `duplicatedBy` (optional, array of strings): Any other games that
          also have the same file path.
    * `registry` (map):
      * Each key is a registry path, and each value is a map with these fields:
        * `failed` (optional, boolean): Whether this entry failed to process.
        * `duplicatedBy` (optional, array of strings): Any other games that
          also have the same registry path.

Note that, in some error conditions, there may not be any JSON output,
so you should check if stdout was blank before trying to parse it.

Example:

```json
{
  "errors": {
    "someGamesFailed": true,
  },
  "overall": {
    "totalGames": 2,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 100,
  },
  "games": {
    "Game 1": {
      "decision": "Processed",
      "files": {
        "/games/game1/save.json": {
          "bytes": 100
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Software/Game1": {
          "failed": true
        }
      }
    },
    "Game 2": {
      "decision": "Ignored",
      "files": {
        "/games/game2/save.json": {
          "bytes": 50
        }
      },
      "registry": {}
    }
  }
}
```

</details>

### Configuration
Ludusavi stores its configuration in the following locations:

* Windows: `%APPDATA%/ludusavi`
* Linux: `$XDG_CONFIG_HOME/ludusavi` or `~/.config/ludusavi`
* Mac: `~/Library/Application Support/ludusavi`

Alternatively, if you'd like Ludusavi to store its configuration in the same
place as the executable, then simply create a file called `ludusavi.portable`
in the directory that contains the executable file. You might want to do that
if you're going to run Ludusavi from a flash drive on multiple computers.

If you're using the GUI, then it will automatically update the config file
as needed, so you don't need to worry about its content. However, if you're
using the CLI exclusively, then you'll need to edit `config.yaml` yourself.

Here are the available settings (all are required unless otherwise noted):

<details>
<summary>Click to expand</summary>

* `manifest` (map):
  * `url` (string): Where to download the primary manifest.
  * `etag` (string or null): An identifier for the current version of the manifest.
    This is generated automatically when the manifest is updated.
* `roots` (list):
  * Each entry in the list should be a map with these fields:
    * `path` (string): Where the root is located on your system.
    * `store` (string): Game store associated with the root. Valid options:
      `epic`, `gog`, `gogGalaxy`, `microsoft`, `origin`, <!-- `prime`, -->
      `steam`, `uplay`, `otherHome`, `otherWine`, `other`
* `backup` (map):
  * `path` (string): Full path to a directory in which to save backups.
    This can be overridden in the CLI with `--path`.
  * `ignoredGames` (optional, array of strings): Names of games to skip when backing up.
    This can be overridden in the CLI by passing a list of games.
  * `merge` (optional, boolean): Whether to merge save data into the target
    directory rather than deleting the directory first. Default: false.
  * `filter` (optional, map):
    * `excludeOtherOsData` (optional, boolean): If true, then the backup should
      exclude any files that have only been confirmed for a different operating
      system than the one you're using. On Linux, Proton saves will still be
      backed up regardless of this setting. Default: false.
    * `excludeStoreScreenshots` (optional, boolean): If true, then the backup
      should exclude screenshots from stores like Steam. Default: false.
* `restore` (map):
  * `path` (string): Full path to a directory from which to restore data.
    This can be overridden in the CLI with `--path`.
  * `ignoredGames` (optional, list of strings): Names of games to skip when restoring.
    This can be overridden in the CLI by passing a list of games.
  * `redirects` (optional, list):
    * Each entry in the list should be a map with these fields:
      * `source` (string): The original location when the backup was performed.
      * `target` (string): The new location.
* `customGames` (optional, list):
  * Each entry in the list should be a map with these fields:
    * `name` (string): Name of the game.
    * `files` (optional, list of strings): Any files or directories you want
      to back up.
    * `registry` (optional, list of strings): Any registry keys you want to back up.

Example:

```yaml
manifest:
  url: "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml"
  etag: null
roots:
  - path: "D:/Steam"
    store: steam
backup:
  path: ~/ludusavi-backup
restore:
  path: ~/ludusavi-backup
```

</details>

Ludusavi also stores `manifest.yaml` (info on what to back up) here.
You should not modify that file, because Ludusavi will overwrite your changes
whenever it downloads a new copy.

## Comparison with other tools
There are other excellent backup tools available, but not a singular
cross-platform and cross-store solution:

* [GameSave Manager](https://www.gamesave-manager.com) (as of v3.1.471.0):
  * Only supports Windows.
  * Closed source, so the community cannot contribute improvements.
  * Interface can be slow or unresponsive; e.g., when (de)selecting all checkboxes,
    it takes half a second per checkbox for them all to toggle.
  * No command line interface.
  * Can create symlinks for games and game data (not currently supported by Ludusavi).
* [Gaming Backup Multitool for Linux](https://supremesonicbrazil.gitlab.io/gbml-web) (as of v1.4.0.0):
  * Only supports Linux and Steam.
  * Database is not actively updated (as of 2020-06-20, the last update was 2018-06-05).
  * No command line interface.
* [Game Backup Monitor](https://mikemaximus.github.io/gbm-web) (as of v1.2.2):
  * Does not support Mac.
  * Database only covers 479 games (as of 2020-09-30).
  * No command line interface.
  * Can compress and keep multiple copies of saves (not currently supported by Ludusavi).
  * Can automatically back up saves for a game after you play it
    (Ludusavi can only do that in conjunction with a launcher like Playnite).

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).

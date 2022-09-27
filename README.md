# ![Logo](assets/icon.svg) Ludusavi
Ludusavi is a tool for backing up your PC video game save data,
written in [Rust](https://www.rust-lang.org).
It is cross-platform and supports multiple game stores.

## Features
* Ability to back up data from more than 10,000 games plus your own custom entries.
* Backup and restore for Steam as well as other game libraries.
* Both a graphical interface and command line interface for scripting.
  Tab completion is available for Bash, Fish, Zsh, PowerShell, and Elvish.
* Support for:
  * Saves that are stored as files and in the Windows registry.
  * Proton saves with Steam.
  * Steam screenshots.
* Available as a [Playnite](https://playnite.link) extension:
  https://github.com/mtkennerly/ludusavi-playnite
* Works on the Steam Deck.
  * For desktop mode, set the `WINIT_X11_SCALE_FACTOR` environment variable to `1`.

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

* For Linux, Ludusavi is available on [Flathub](https://flathub.org/apps/details/com.github.mtkennerly.ludusavi).
  Note that it has limited file system access by default (`~` and `/run/media`).
  If you'd like to enable broader access, [see here](https://github.com/flathub/com.github.mtkennerly.ludusavi/blob/master/README.md).

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
### Roots
Roots are folders that Ludusavi can check for additional game data. When you
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

You may use globs in root paths to identify multiple roots at once.

### Backup retention
You can configure how many backups to keep by pressing the gear icon on the backup screen.

A differential backup contains just the changed files since the last full backup,
and differential backup retention is tied to the associated full backup as well.

If you configure 2 full and 2 differential, then Ludusavi will create 2 differential backups
for each full backup, like so:

* Backup #1: full
  * Backup #2: differential
  * Backup #3: differential
* Backup #4: full
  * Backup #5: differential
  * Backup #6: differential

When backup #7 is created, because the full retention is set to 2,
Ludusavi will delete backups 1 through 3.

### Selective scanning
Once you've done at least one full scan (via the preview/backup buttons),
Ludusavi will remember the games it found and show them to you the next time you run the program.
That way, you can selectively preview or back up a single game without doing a full scan.
Use the three-dot menu next to each game's title to operate on just that one game.

You can also use keyboard shortcuts to swap the three-dot menu with some specific buttons:

* preview: shift
* backup/restore: ctrl (Mac: cmd)
* backup/restore without confirmation: ctrl + alt (Mac: cmd + option)

### Backup structure
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

### Search
You can click the search icon and enter some text to just see games with
matching names. Note that this only affects which games you see in the list,
but Ludusavi will still back up the full set of games.

Sorting options are also available while the search bar is open.

### Duplicates
You may see a "duplicates" badge next to some games. This means that some of
the same files were also backed up for another game. That could be intentional
(e.g., an HD remaster may reuse the original save locations), but it could
also be a sign of an issue in the manifest data. You can expand the game's
file list to see which exact entries are duplicated.

### Restoration redirect
You can use redirects to restore to a different location than the original file.
Click `add redirect` on the restore screen, and then enter both the old and new location.
For example, if you backed up some saves from `C:/Games`, but then you moved it to `D:/Games`,
then you would put `C:/Games` as the source and `D:/Games` as the target.

Tip: As you're editing your redirects, try running a preview and expanding some
games' file lists. This will show you in real time what effect your redirects
will have when you perform the restore for real.

### Custom games
You can create your own game save definitions on the `custom games` screen.
If the game name exactly matches a known game, then your custom entry will override it.

For file paths, you can click the browse button to quickly select a folder.
The path can be a file too, but the browse button only lets you choose
folders at this time. You can just type in the file name afterwards.
You can also use [globs](https://en.wikipedia.org/wiki/Glob_(programming))
(e.g., `C:/example/*.txt` selects all TXT files in that folder)
and the placeholders defined in the
[Ludusavi Manifest format](https://github.com/mtkennerly/ludusavi-manifest).

### Backup exclusions
Backup exclusions let you set paths and registry keys to completely ignore
from all games. They will not be shown at all during backup scans.

Configure exclusions on the `other` screen.

### Command line
Run `ludusavi --help` for the full CLI usage information.

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

Ludusavi also stores `manifest.yaml` (info on what to back up) here.
You should not modify that file, because Ludusavi will overwrite your changes
whenever it downloads a new copy.

### Logging
Log files are stored in the config folder (see above).
By default, only warnings and errors are logged,
but you can customize this by setting the `RUST_LOG` environment variable
(e.g., `RUST_LOG=ludusavi=debug`).
The most recent 5 log files are kept, rotating on app launch or when a log reaches 10 MiB.

## Interfaces
### CLI API
CLI mode defaults to a human-readable format, but you can switch to a
machine-readable JSON format with the `--api` flag.

<details>
<summary>Click to expand</summary>

For the `backup`/`restore` commands:

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
        * `ignored` (optional, boolean): Whether this entry was ignored.
        * `bytes` (number): Size of the file.
        * `originalPath` (optional, string): If the file was restored to a
          redirected location, then this is its original path.
        * `duplicatedBy` (optional, array of strings): Any other games that
          also have the same file path.
    * `registry` (map):
      * Each key is a registry path, and each value is a map with these fields:
        * `failed` (optional, boolean): Whether this entry failed to process.
        * `ignored` (optional, boolean): Whether this entry was ignored.
        * `duplicatedBy` (optional, array of strings): Any other games that
          also have the same registry path.

The `backups` command is similar, but without `overall`, and with each game containing
`{"backups": [ {"name": <string>, "when": <string>} ]}`

Note that, in some error conditions, there may not be any JSON output,
so you should check if stdout was blank before trying to parse it.
If the command line input cannot be parsed, then the output will not be
in a stable format.

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

### Configuration file
Here are the available settings in `config.yaml` (all are required unless otherwise noted):

<details>
<summary>Click to expand</summary>

* `manifest` (map):
  * `url` (string): Where to download the primary manifest.
  * `etag` (string or null): An identifier for the current version of the manifest.
    This is generated automatically when the manifest is updated.
* `language` (string, optional): Display language. Valid options:
  `en-US` (English, default), `fil-PH` (Filipino), `de-DE` (German), `it-IT` (Italian), `pt-BR` (Brazilian Portuguese), `pl-PL` (Polish), `es-ES` (Spanish).

  Experimental options that currently have graphical display issues:
  `ar-SA` (Arabic), `zh-Hans` (Simplified Chinese), `ko-KR` (Korean).
* `theme` (string, optional): Visual theme. Valid options:
  `light` (default), `dark`.
* `roots` (list):
  * Each entry in the list should be a map with these fields:
    * `path` (string): Where the root is located on your system.
    * `store` (string): Game store associated with the root. Valid options:
      `epic`, `gog`, `gogGalaxy`, `microsoft`, `origin`, `prime`,
      `steam`, `uplay`, `otherHome`, `otherWine`, `other`
* `backup` (map):
  * `path` (string): Full path to a directory in which to save backups.
    This can be overridden in the CLI with `--path`.
  * `ignoredGames` (optional, array of strings): Names of games to skip when backing up.
    This can be overridden in the CLI by passing a list of games.
  * `merge` (optional, boolean): Whether to merge save data into the target
    directory rather than deleting the directory first. Default: true.
  * `filter` (optional, map):
    * `excludeStoreScreenshots` (optional, boolean): If true, then the backup
      should exclude screenshots from stores like Steam. Default: false.
    * `ignoredPaths` (list of strings): Globally ignored paths.
    * `ignoredRegistry` (list of strings): Globally ignored registry keys.
  * `toggledPaths` (map): Paths overridden for inclusion/exclusion in the backup.
    Each key is a game name, and the value is another map. In the inner map,
    each key is a path, and the value is a boolean (true = included).
    Settings on child paths override settings on parent paths.
  * `toggledRegistry` (map): Same as `toggledPaths`, but for registry entries.
  * `sort` (map):
    * `key` (string): One of `name`, `size`.
    * `reversed` (boolean): If true, sort reverse alphabetical or from the largest size.
  * `retention` (map):
    * `full` (integer): Full backups to keep. Range: 1-255.
    * `differential` (integer): Full backups to keep. Range: 0-255.
  * `format` (map):
    * `chosen` (string): One of `simple`, `zip`.
    * `zip` (map): Settings for the zip format.
      * `compression` (string): One of `none`, `deflate`, `bzip2`, `zstd`.
* `restore` (map):
  * `path` (string): Full path to a directory from which to restore data.
    This can be overridden in the CLI with `--path`.
  * `ignoredGames` (optional, list of strings): Names of games to skip when restoring.
    This can be overridden in the CLI by passing a list of games.
  * `redirects` (optional, list):
    * Each entry in the list should be a map with these fields:
      * `source` (string): The original location when the backup was performed.
      * `target` (string): The new location.
  * `sort` (map):
    * `key` (string): One of `name`, `size`.
    * `reversed` (boolean): If true, sort reverse alphabetical or from the largest size.
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
  * Database only covers 479 games (as of 2020-09-30), although it can also import
    the Ludusavi manifest starting in 1.3.1.
  * No command line interface.
  * Can automatically back up saves for a game after you play it
    (Ludusavi can only do that in conjunction with a launcher like Playnite).

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).

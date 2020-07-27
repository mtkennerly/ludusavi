# ![Logo](assets/icon.png) Ludusavi
[![Version](https://img.shields.io/crates/v/ludusavi)](https://crates.io/crates/ludusavi)
[![License: MIT](https://img.shields.io/badge/license-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Ludusavi is a tool for backing up your PC video game save data,
written in [Rust](https://www.rust-lang.org).
It is cross-platform and supports multiple game stores.

This tool uses the [Ludusavi Manifest](https://github.com/mtkennerly/ludusavi-manifest)
for info on what to back up, and it will automatically download the latest version of
the primary manifest. To add or update game entries in the primary manifest, please refer
to that project. Data is ultimately sourced from [PCGamingWiki](https://www.pcgamingwiki.com/wiki/Home),
so you are encouraged to contribute any new or fixed data back to the wiki itself.

## Features
* Ability to back up data from more than 7,000 games plus your own custom entries.
* Backup and restore for Steam as well as other game libraries.
* Preview of the backup/restore before actually performing it.
* Both a graphical interface and command line interface for scripting.
* Support for:
  * Saves that are stored as files and in the Windows registry.
  * Proton saves with Steam.
  * Steam screenshots.

## Demo
### GUI
> ![GUI demo of previewing a backup](docs/demo-gui.gif)

### CLI
> ![CLI demo of previewing a backup](docs/demo-cli.gif)

## Installation
### Requirements
Ludusavi is available for Windows, Linux, and Mac. However, your computer must
support one of these graphics systems: Vulkan, DirectX (11 or 12), or Metal.
(Experimental builds with OpenGL support are also available - give them a try
if the standard builds don't work on your system.)

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
### CLI
Run `ludusavi --help` for the full usage information.

### GUI
#### Backup mode
* This is the default mode when you open the program.
* You can press `preview` to see what the backup will include,
  without actually performing it.
* You can press `back up` to perform the backup for real.
  * If the target folder already exists, it will be deleted first,
    then recreated.
  * Within the target folder, for every game with data to back up,
    a subfolder will be created with the game's name encoded as
    [Base64](https://en.wikipedia.org/wiki/Base64).
    For example, files for `Celeste` would go into a folder named `Q2VsZXN0ZQ==`.
  * Within each game's backup folder, any relevant files will be stored with
    their name as the Base64 encoding of the full path to the original file.
    For example, `D:/Steam/steamapps/common/Celeste/Saves/0.celeste` would be
    backed up as `RDovU3RlYW0vc3RlYW1hcHBzL2NvbW1vbi9DZWxlc3RlL1NhdmVzLzAuY2VsZXN0ZQ==`.
  * If the game has save data in the registry and you are using Windows, then
    the game's backup folder will also contain an `other/registry.yaml` file.
    If you are using Steam and Proton instead of Windows, then the Proton `*.reg`
    files will be backed up like other game files.
* Roots are folders that Ludusavi can check for additional game data. When you
  first run Ludusavi, it will try to find some common roots on your system, but
  you may end up without any configured. You can click `add root` to configure
  as many as you need, along with the root's type:
  * For a Steam root, this should be the folder containing the `steamapps` and
    `userdata` subdirectories. Here are some common/standard locations:
    * Windows: `C:/Program Files (x86)/Steam`
    * Linux: `~/.steam/steam`
  * For the "other" root type, it should be a folder whose direct children are
    individual games. For example, in the Epic Games store, this would be
    what you choose as the "install location" for your games (e.g., if you choose
    `D:/Epic` and it creates a subfolder for `D:/Epic/Celeste`, then the root
    would be `D:/Epic`).
* To select/deselect specific games, you can run a preview, then click the
  checkboxes by each game. You can also press the `deselect all` button
  (when all games are selected) or the `select all` button (when at least
  one game is deselected) to quickly toggle all of them at once.
  Ludusavi will remember your most recent checkbox settings.

#### Restore mode
* Switch to restore mode by clicking the `restore mode` button.
* You can press `preview` to see what the restore will include,
  without actually performing it.
* You can press `restore` to perform the restore for real.
  * For all the files in the source directory, they will be decoded as Base64
    to get the target path and then copied to that location. Any necessary
    parent directories will be created as well before the copy, but if the
    directories already exist, their current files will be left alone (other
    than overwriting the ones that are being restored from the backup).
* You can use redirects to restore to a different location than the original file.
  Click `add redirect`, and then enter both the old and new location. For example,
  if you backed up some saves from `C:/Games`, but then you moved it to `D:/Games`,
  then you would put `C:/Games` as the source and `D:/Games` as the target.

  Tip: As you're editing your redirects, try running a preview and expanding some
  games' file lists. This will show you in real time what effect your redirects
  will have when you perform the restore for real.
* You can select/deselect specific games in restore mode just like you can in
  backup mode. The checkbox settings are remembered separately for both modes.

#### Custom games
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
  custom entry will override it.

### Configuration
Ludusavi stores its configuration in `~/.config/ludusavi` (Windows: `C:/Users/<your-name>/.config/ludusavi`).
If you're using the GUI, you don't need to worry about this at all,
since the GUI will automatically update the config file as needed.
However, if you're using the CLI, you'll need to edit `config.yaml` directly.
Here are the available settings (all are required unless otherwise noted):

* `manifest` (map):
  * `url` (string): Where to download the primary manifest.
  * `etag` (string or null): An identifier for the current version of the manifest.
    This is generated automatically when the manifest is updated.
* `roots` (list):
  * Each entry in the list should be a map with these fields:
    * `path` (string): Where the root is located on your system.
    * `store` (string): Game store associated with the root.
      Valid options: `steam`, `other`
* `backup` (map):
  * `path` (string): Full path to a directory in which to save backups.
    This can be overridden in the CLI with `--path`.
  * `ignoredGames` (optional, array of strings): Names of games to skip when backing up.
    This can be overridden in the CLI by passing a list of games.
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
* [Gaming Backup Multitool for Linux](https://supremesonicbrazil.gitlab.io/gbml-web) (as of v1.4.0.0):
  * Only supports Linux and Steam.
  * Database is not actively updated (as of 2020-06-20, the last update was 2018-06-05).
  * No command line interface.

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).

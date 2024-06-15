# ![Logo](assets/icon.svg) Ludusavi
Ludusavi is a tool for backing up your PC video game save data,
written in [Rust](https://www.rust-lang.org).
It is cross-platform and supports multiple game stores.

## Features
* Ability to back up data from more than 19,000 games plus your own custom entries.
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
* Your system must support one of DirectX, Vulkan, or Metal.
  (If not, set the `ICED_BACKEND` environment variable to `tiny-skia` to use the software renderer.)

### Methods
You can install Ludusavi one of these ways:

* Download the executable for your operating system from the
  [releases page](https://github.com/mtkennerly/ludusavi/releases).
  It's portable, so you can simply download it and put it anywhere
  on your system.
  **If you're unsure, choose this option.**

* On Windows, you can use [Winget](https://github.com/microsoft/winget-cli).

  * To install: `winget install -e --id mtkennerly.ludusavi`
  * To update: `winget upgrade -e --id mtkennerly.ludusavi`

* On Windows, you can use [Scoop](https://scoop.sh).

  * To install: `scoop bucket add extras && scoop install ludusavi`
  * To update: `scoop update && scoop update ludusavi`

* For Linux, Ludusavi is available on [Flathub](https://flathub.org/apps/details/com.github.mtkennerly.ludusavi).
  Note that it has limited file system access by default (`~` and `/run/media`).
  If you'd like to enable broader access, [see here](https://github.com/flathub/com.github.mtkennerly.ludusavi/blob/master/README.md).

* If you have [Rust](https://www.rust-lang.org), you can use Cargo.

  * To install or update: `cargo install --locked ludusavi`

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
<!-- These anchors are kept for compatibility with old section headers. -->
<a name="backup-exclusions"></a>
<a name="backup-retention"></a>
<a name="backup-structure"></a>
<a name="backup-validation"></a>
<a name="cli-api"></a>
<a name="cloud-backup"></a>
<a name="command-line"></a>
<a name="configuration"></a>
<a name="configuration-file"></a>
<a name="custom-games"></a>
<a name="duplicates"></a>
<a name="environment-variables"></a>
<a name="filter"></a>
<a name="game-launch-wrapping"></a>
<a name="logging"></a>
<a name="redirects"></a>
<a name="roots"></a>
<a name="selective-scanning"></a>

Detailed help documentation is available for several topics.

### General
* [Backup automation](/docs/help/backup-automation.md)
* [Backup exclusions](/docs/help/backup-exclusions.md)
* [Backup retention](/docs/help/backup-retention.md)
* [Backup validation](/docs/help/backup-validation.md)
* [Cloud backup](/docs/help/cloud-backup.md)
* [Custom games](/docs/help/custom-games.md)
* [Duplicates](/docs/help/duplicates.md)
* [Filter](/docs/help/filter.md)
* [Game launch wrapping](/docs/help/game-launch-wrapping.md)
* [Redirects](/docs/help/redirects.md)
* [Roots](/docs/help/roots.md)
* [Selective scanning](/docs/help/selective-scanning.md)

### Interfaces
* [Application folder](/docs/help/application-folder.md)
* [Backup structure](/docs/help/backup-structure.md)
* [Command line](/docs/help/command-line.md)
* [Configuration file](/docs/help/configuration-file.md)
* [Environment variables](/docs/help/environment-variables.md)
* [Logging](/docs/help/logging.md)

## Community

The community has created some additional resources you may find useful.
Please note that this is not an exhaustive list
and that these projects are not officially affiliated with Ludusavi itself:

* Secondary manifests:
  * https://github.com/BloodShed-Oni/ludusavi-extra-manifests
  * https://github.com/hblamo/ludusavi-emudeck-manifest
* Plugins for Decky Loader on Steam Deck:
  * https://github.com/GedasFX/decky-ludusavi
* Plugins for VS Code:
  * https://marketplace.visualstudio.com/items?itemName=claui.ludusavi

## Comparison with other tools
There are other excellent backup tools available, but not a singular
cross-platform and cross-store solution:

* [GameSave Manager](https://www.gamesave-manager.com) (as of v3.1.512.0):
  * Only supports Windows.
  * Much slower than Ludusavi. On the same hardware and with default settings,
    an initial scan of the whole system takes 2 minutes in GSM versus 10 seconds in Ludusavi.
    Performing a backup immediately after that scan takes 4 minutes 16 seconds in GSM versus 4.5 seconds in Ludusavi.
    In this test, GSM found 257 games with 2.84 GB, and Ludusavi found 297 games with 2.95 GiB.
  * Closed source, so the community cannot contribute improvements.
  * Interface can be slow or unresponsive.
    For example, when clicking "select all / de-select all", each checkbox has to individually toggle itself.
    With 257 games, this means you end up having to wait around 42 seconds.
  * Minimal command line interface.
  * Can create symlinks for games and game data.
    Ludusavi does not support this.
* [Game Backup Monitor](https://mikemaximus.github.io/gbm-web) (as of v1.2.2):
  * Does not support Mac.
  * Database only covers 577 games (as of 2022-11-16), although it can also import
    the Ludusavi manifest starting in 1.3.1.
  * No command line interface.
  * Can automatically back up saves for a game after you play it.
    Ludusavi can only do that in conjunction with a launcher like Playnite.
* [Gaming Backup Multitool for Linux](https://supremesonicbrazil.gitlab.io/gbml-web) (as of v1.4.0.0):
  * Only supports Linux and Steam.
  * Database is not actively updated. As of 2022-11-16, the last update was 2018-06-05.
  * No command line interface.

## Troubleshooting
* The window content is way too big and goes off screen.
  * Try setting the `WINIT_X11_SCALE_FACTOR` environment variable to `1`.
    Flatpak installs will have this set automatically.
* The file/folder picker doesn't work.
  * **Linux:** Make sure that you have Zenity or kdialog installed and available on the `PATH`.
    The `DISPLAY` environment variable must also be set.
  * **Steam Deck:** Use desktop mode instead of game mode.
  * **Flatpak:** The `DISPLAY` environment variable may not be getting passed through to the container.
    This has been observed on GNOME systems.
    Try running `flatpak run --nosocket=fallback-x11 --socket=x11 com.github.mtkennerly.ludusavi`.
* On Windows 11, when I open the GUI, a console window also stays open.
  * This is a limitation of the new Windows Terminal app (https://github.com/microsoft/terminal/issues/14416).
    It should be fixed once Windows Terminal v1.17 is released.
    In the meantime, you can work around it by opening Windows Terminal from the Start Menu,
    opening its settings, and changing the "default terminal application" to "Windows Console Host".
* The GUI won't launch.
  * There may be an issue with your graphics drivers/support.
    Try using the software renderer instead by setting the `ICED_BACKEND` environment variable to  `tiny-skia`.

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).

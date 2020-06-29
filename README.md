# Ludusavi
**This project is still a prototype!**

Ludusavi is a tool for backing up your PC video game save data, written in Rust.
It is cross-platform and supports multiple game stores.

This tool uses the [Ludusavi Manifest](https://github.com/mtkennerly/ludusavi-manifest)
for info on what to back up, and it will automatically download the latest version of
the primary manifest. To add or update game entries in the primary manifest, please refer
to that project.

## Features
* Backup and restore for Steam as well as other game libraries.
* Preview the backup/restore before actually performing it.
* Support for Proton saves with Steam.

Planned for the future:

* (De)selecting specific games for backup/restore.
* Restoring to different locations.
* Backing up saves from the Windows registry.
* CLI mode.

## Comparison with other tools
There are other excellent backup tools available, but not a singular
cross-platform and cross-store solution:

* [GameSave Manager](https://www.gamesave-manager.com):
  * Only supports Windows and Steam.
  * Closed source, so the community cannot contribute improvements.
  *
* [Gaming Backup Multitool for Linux](https://supremesonicbrazil.gitlab.io/gbml-web)
  * Only supports Linux and Steam.
  * Database is not actively updated (as of 2020-06-20, the last update was 2018-06-05).

## Development
Please refer to [CONTRIBUTING.md](./CONTRIBUTING.md).

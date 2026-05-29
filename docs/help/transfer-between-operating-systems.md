# Transfer between operating systems
Although Ludusavi itself runs on Windows, Linux, and Mac,
it does not automatically support backing up on one OS and restoring on another
for native platform paths.

This is a complex problem to solve because
games do not necessarily store data in the same way on each OS.
Ludusavi only knows where each game stores its data on a given OS,
but does not know which save locations correspond to each other,
or even if any of them do correspond.
Some games store data in completely different and incompatible ways on different OSes.

## Windows and Wine/Proton (supported)

Ludusavi supports portable backups between Windows and Wine/Proton prefixes.
When a game's saves are stored under standard Windows locations
(Documents, AppData, ProgramData, etc.),
Ludusavi automatically uses a portable semantic path format
that works across Windows and Wine environments.

This means:

- **Windows → Wine/Proton:** Back up on Windows, restore into a Wine prefix on Linux
  without needing per-game redirects.
- **Wine/Proton → Windows:** Back up from a Wine prefix on Linux, restore on Windows.
  The backup identity is the same regardless of the Wine prefix location or username.

Usernames (Windows `Alice`, Wine `steamuser`, SteamOS `deck`) and
Wine prefix paths are intentionally excluded from the backup identity.

### Requirements

- The game must store saves under a recognized Windows location
  (Documents, AppData, LocalAppData, Saved Games, Public, ProgramData, Windows directory).
- When restoring into Wine on Linux, you need a configured Wine prefix
  (via game-specific prefix roots, Heroic/Lutris/Steam Proton discovery,
  or the `--wine-prefix` CLI option).
- If a game should always restore into one prefix, set
  `restore.preferredWinePrefixes.<game>.path` in `config.yaml`. A CLI
  `--wine-prefix` value must match that saved preference for the game;
  otherwise Ludusavi stops instead of silently restoring into a different
  prefix.

### Limitations

- Steam userdata paths use store-specific identity
  (different Steam accounts = different save entries).
- Non-C drive paths require matching `dosdevices` mappings on the target.
- Registry-based saves are not yet portable across Windows and Wine.
- Native Windows to native Linux path equivalence is not supported
  unless the manifest explicitly declares the relationship.

## Native cross-OS (limited)

For native Windows, Linux, and macOS paths that are not through Wine,
Ludusavi cannot automatically determine if saves are equivalent.
In simple cases, you may be able to configure [redirects](/docs/help/redirects.md)
to translate between specific Windows and Linux paths,
but this would generally require multiple redirects tailored to each game.

You can follow this ticket for future updates on native cross-OS support:
https://github.com/mtkennerly/ludusavi/issues/194

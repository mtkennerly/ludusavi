# Roots
Roots are folders that Ludusavi can check for additional game data. When you
first run Ludusavi, it will try to find some common roots on your system, but
you may end up without any configured. These are listed on the "other" screen,
where you can use the plus button in the roots section to configure as many as you need,
along with the root's type:

* For a Steam root, this should be the folder containing the `steamapps` and
  `userdata` subdirectories. Here are some common/standard locations:
  * Windows: `C:/Program Files (x86)/Steam`
  * Linux: `~/.steam/steam`

  On Linux, for games that use Proton, Ludusavi will back up the `*.reg` files
  if the game is known to have registry-based saves.

  On Linux, if you've used Steam's "add a non-Steam game" feature,
  then Ludusavi will also back up any Proton save data for those games.
  This requires the shortcut name in Steam to match the title by which Ludusavi knows the game
  (i.e., the title of its PCGamingWiki article).
* For a Heroic root, this should be the folder containing the `gog_store`
  and `GamesConfig` subdirectories.

  Ludusavi can find GOG, Epic, Amazon, and sideloaded game saves in Heroic's game install folders.
  On Linux, Ludusavi can also find saves in Heroic's Wine, Proton, and Lutris prefixes.

  When using Wine prefixes with Heroic, Ludusavi will back up the `*.reg` files
  if the game is known to have registry-based saves.
* For a Legendary root, this should be the folder containing `installed.json`.
  Currently, Ludusavi cannot detect Wine prefixes for Legendary roots.
* For a Lutris root, this should be the folder containing the `games` subdirectory.

  Ludusavi expects the game YAML files to contain a few fields,
  particularly `name` and either `game.working_dir` or `game.exe`.
  Games will be skipped if they don't have the necessary fields.
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
* The Windows, Linux, and Mac drive roots can be used
  to make Ludusavi scan external hard drives with a separate OS installation.
  For example, let's say you had a Windows laptop that broke,
  but you recovered the hard drive and turned it into an external drive.
  You could add it as a Windows drive root to make Ludusavi scan it.

  In this case, Ludusavi can only look for normal/default locations of system folders.
  Ludusavi will not be able to use the Windows API or check `XDG` environment variables
  to detect alternative folder locations (e.g., if you've moved the `Documents` folder).

You may use [globs] in root paths to identify multiple roots at once.
If you have a folder name that contains a special glob character,
you can escape it by wrapping it in brackets (e.g., `[` becomes `[[]`).

The order of the configured roots is not significant.
The only case where it may make a difference is if Ludusavi finds secondary manifests (`.ludusavi.yaml` files)
*and* those manfiests contain overlapping entries for the same game,
in which case Ludusavi will merge the data together in the order that it finds them.

[globs]: https://en.wikipedia.org/wiki/Glob_(programming)

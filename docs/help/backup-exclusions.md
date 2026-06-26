# Backup exclusions
Backup exclusions let you set paths and registry keys to completely ignore
from all games. They will not be shown at all during backup scans.

Configure exclusions on the "other" screen.

For excluded file paths, you can use glob syntax.
For example, to exclude all files named `remotecache.vdf`, you would specify `**/remotecache.vdf`.

## Cloud-supported games
Ludusavi can skip games that have cloud save support on selected stores.
You can configure this on the "other" screen.

For example, if you enable cloud exclusions for Steam, then Ludusavi will skip
games whose manifest says they support Steam Cloud. This can apply even if you
installed the game through another launcher, because the cloud metadata belongs
to the game itself, not just to your installed copy.

This can be confusing when a game is installed and its saves are valid, but it
does not appear in a full backup preview. For example:

* You install a Windows game through Heroic.
* Ludusavi detects the game and can find its save files.
* The game also has Steam Cloud metadata in Ludusavi's manifest.
* Steam cloud exclusions are enabled.
* The game has no previous Ludusavi backup yet.

In that case, a full backup preview may not list the game, because the cloud
exclusion applies to first-time backups. You can still preview or back up the
game explicitly.

From the CLI, preview a specific game like this:

```sh
ludusavi backup --preview "Deus Ex: Mankind Divided"
```

Or back it up explicitly like this:

```sh
ludusavi backup "Deus Ex: Mankind Divided"
```

To include the game in full backup previews, disable the cloud exclusion for
that store, or disable cloud exclusions entirely.

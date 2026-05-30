# Configuration
Ludusavi stores its configuration in the [application folder](/docs/help/application-folder.md),
in a file named `config.yaml`.

If you're using the GUI, then it will automatically update the config file
as needed, so you don't need to worry about its content. However, if you're
using the CLI exclusively, then you'll need to edit `config.yaml` yourself.

## Schema
[docs/schema/config.yaml](/docs/schema/config.yaml)

## Example
```yaml
manifest:
  url: "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml"
roots:
  - path: "D:/Steam"
    store: steam
backup:
  path: ~/ludusavi-backup
restore:
  path: ~/ludusavi-backup
  preferredWinePrefixes:
    "Example Game":
      path: ~/Games/Prefixes/Example Game
      wineUser: steamuser
```

`restore.preferredWinePrefixes` is optional. Use it when a portable Windows
backup should always restore into a specific Wine/Proton prefix for a game.
The CLI `--wine-prefix` option is still available as a per-command override,
but Ludusavi rejects it if it conflicts with a saved preference for the game.

`restore.winePrefix` is an optional global fallback Wine prefix for restore.
It is used when no per-game preference, source context, custom game winePrefix,
or launcher-discovered prefix applies.

### Wine prefix resolver priority

When restoring a portable Windows/Wine backup, Ludusavi resolves the target
Wine prefix in this order:

1. CLI `--wine-prefix` override
2. Per-game `restore.preferredWinePrefixes` (from config)
3. Source context from backup metadata (if the prefix path exists on this system)
4. Custom game `winePrefix` (from manifest or custom game config)
5. Launcher-discovered prefixes (Heroic, Lutris, Steam)
6. Global `restore.winePrefix` (from config)
7. Root discovery

If the backup's source context is still valid on the current system, Ludusavi
uses that exact prefix metadata so same-system multi-prefix restores are
unambiguous. If the source prefix no longer exists, the resolver selects a
target prefix from the later priority levels and uses that target prefix's own
Wine user and drive mappings.

At each level, if multiple candidates are found and none can be disambiguated,
Ludusavi returns a `WinePrefixAmbiguity` error listing the candidates so you
can configure a preference.

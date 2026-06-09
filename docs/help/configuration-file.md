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
```

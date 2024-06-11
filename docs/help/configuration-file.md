## Configuration
Ludusavi stores its configuration in the following locations:

* Windows: `%APPDATA%/ludusavi`
* Linux: `$XDG_CONFIG_HOME/ludusavi` or `~/.config/ludusavi`
  * Flatpak: `~/.var/app/com.github.mtkennerly.ludusavi/config/ludusavi`
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

### Schema
[docs/schema/config.yaml](/docs/schema/config.yaml)

### Example
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

# Application folder
Ludusavi stores its configuration/logs/etc in the following locations:

* Windows: `%APPDATA%/ludusavi`
* Linux: `$XDG_CONFIG_HOME/ludusavi` or `~/.config/ludusavi`
  * Flatpak: `~/.var/app/com.github.mtkennerly.ludusavi/config/ludusavi`
* Mac: `~/Library/Application Support/ludusavi`

Alternatively, if you'd like Ludusavi to store its configuration in the same
place as the executable, then simply create a file called `ludusavi.portable`
in the directory that contains the executable file. You might want to do that
if you're going to run Ludusavi from a flash drive on multiple computers.

Ludusavi also stores `manifest.yaml` (info on what to back up) here.
You should not modify that file, because Ludusavi will overwrite your changes
whenever it downloads a new copy.

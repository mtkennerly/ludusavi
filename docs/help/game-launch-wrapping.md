# Game launch wrapping
The [CLI](/docs/help/command-line.md) has a `wrap` command that can be used as a wrapper around launching a game.
When wrapped, Ludusavi will restore data for the game first, launch it, and back up after playing.
If you want to use this feature, you must manually configure your game launcher app to use this command.

In general, you can set your launcher to run `ludusavi wrap --name "GAME_NAME" -- GAME_INVOCATION`.
Some specific launchers have built-in support (see below) to make this easier.

On Linux, this feature works best with a standalone copy of Ludusavi,
rather than the Flatpak version.
In some cases, the Flatpak environment's constraints may keep Ludusavi from launching the game.

## Steam
* Right click on a game in your Steam library and click `properties`.
* In the popup window, set the launch options like so:

  `C:\ludusavi.exe wrap --infer steam --gui -- %command%`

  (Use the actual path to your copy of `ludusavi.exe` instead of `C:\ludusavi.exe`)

You must do this for each game individually.

### Non-Steam shortcuts
As of 2024-12-27,
non-Steam games added as shortcuts in Steam won't work with the above method.
Instead, you have to flip the target and launch option fields like so:

* Set the `target` to `C:\ludusavi.exe` (path to your Ludusavi executable)
* Set the launch options to `wrap --name "GAME NAME" --gui -- "C:\path\to\game.exe"`

On the Steam Deck in game mode,
using the Steam overlay to quit the game will also quit the Ludusavi wrapper,
preventing the post-game prompt to back up the save data.
To avoid this, you should use the game's built-in quit option instead.
Currently, this has only been confirmed to happen in game mode, not desktop mode or Big Picture.

## Heroic
### Heroic 2.9.2+ (Linux example)
Create a file named `ludusavi-wrap.sh` with this content:

```
#!/bin/sh
ludusavi --config $HOME/.config/ludusavi wrap --gui --infer heroic -- "$@"
```

Mark the file as executable and set it as a wrapper within Heroic.
You must set it as a wrapper for each game already installed individually.

Note that the `--config` option is required because Heroic overrides the `XDG_CONFIG_HOME` environment variable,
which would otherwise prevent Ludusavi from finding its configuration.

## Playnite
For Playnite, you should use the [official plugin](https://github.com/mtkennerly/ludusavi-playnite),
which provides deeper integration between Playnite and Ludusavi.

That being said, you *can* set up a wrapper script instead if you prefer.
You have to configure two scripts:
one when the game starts, and one when the game stops.

In Playnite, navigate to settings -> scripts -> game scripts,
and configure the following:

* Execute before starting a game
  (if you want Ludusavi to restore your latest backup):
  ```
  C:\ludusavi.exe restore --force "$game"
  ```
* Execute after exiting a game
  (if you want Ludusavi to make a new backup):
  ```
  C:\ludusavi.exe backup --force "$game"
  ```

(Use the actual path to your copy of `ludusavi.exe` instead of `C:\ludusavi.exe`)

## Lutris
Ludusavi can be configured globally for all games in [Lutris](https://lutris.net/).
Note these instructions are for Lutris and ludusavi which are installed directly on the system, not the Flatpak versions.
In Lutris, open the `Global options` tab in the `Preferences` menu.
Enable `Advanced` in the top-right corner.
Scroll down to the `Game execution` section.
In the `Command prefix` entry, enter `ludusavi wrap --infer lutris --gui --`.
If `ludusavi` isn't available on the `PATH`, be sure to fully qualify the path to the `ludusavi` executable.
Finally, hit `Save`.
Ludusavi will now run for every game.

If you'd rather just set this directly in a Lutris config file, add the following to `~/.local/share/lutris/system.yml`.

```yaml
system:
  prefix_command: 'ludusavi wrap --infer lutris --gui --'
```

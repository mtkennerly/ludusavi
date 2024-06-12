# Game launch wrapping
The [CLI](/docs/help/command-line.md) has a `wrap` command that can be used as a wrapper around launching a game.
When wrapped, Ludusavi will restore data for the game first, launch it, and back up after playing.
If you want to use this feature, you must manually configure your game launcher app to use this command.

In general, you can set your launcher to run `ludusavi wrap --name "GAME_NAME" -- GAME_INVOCATION`.
Some specific launchers have built-in support (see below) to make this easier.

## Steam
* Right click on a game in your Steam library and click `properties`.
* In the popup window, set the launch options like so:

  `C:\ludusavi.exe wrap --infer steam --gui -- %command%`

  (Use the actual path to your copy of `ludusavi.exe` instead of `C:\ludusavi.exe`)

You must do this for each game individually.

## Heroic 2.9.2+ (Linux example)
Create a file named `ludusavi-wrap.sh` with this content:

```
$!/bin/sh
ludusavi --config $HOME/.config/ludusavi wrap --gui --infer heroic -- "$@"
```

Mark the file as executable and set it as a wrapper within Heroic.
You must set it as a wrapper for each game already installed individually.

Note that the `--config` option is required because Heroic overrides the `XDG_CONFIG_HOME` environment variable,
which would otherwise prevent Ludusavi from finding its configuration.

## Game launch wrapping
The CLI has a `wrap` command that can be used as a wrapper around launching a game.
When wrapped, Ludusavi will restore data for the game first, launch it, and back up after playing.
If you want to use this feature, you must manually configure your game launcher app to use this command.

If you use Heroic 2.9.2 or newer, you can run `wrap --infer heroic -- GAME_INVOCATION` to automatically check the game name.
For other launcher apps, you can run `wrap --name GAME_NAME -- GAME_INVOCATION`.

### Example with Heroic 2.9.2 on Linux
Create a file named `ludusavi-wrap.sh` with this content:

```
$!/bin/sh
ludusavi --try-manifest-update --config $HOME/.config/ludusavi wrap --gui --infer heroic -- "$@"
```

Mark the file as executable and set it as a wrapper within Heroic.
You must set it as a wrapper for each game already installed individually.

Note that the `--config` option is required because Heroic overrides the `XDG_CONFIG_HOME` environment variable,
which would otherwise prevent Ludusavi from finding its configuration.

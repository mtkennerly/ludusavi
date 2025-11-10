This is the raw help text for the command line interface.

## `--help`
```
Back up and restore PC game saves

Usage: ludusavi.exe [OPTIONS] [COMMAND]

Commands:
  backup
          Back up data
  restore
          Restore data
  complete
          Generate shell completion scripts
  backups
          Show backups
  find
          Find game titles
  manifest
          Options for Ludusavi's data set
  config
          Options for Ludusavi's configuration
  cloud
          Cloud sync
  wrap
          Wrap restore/backup around game execution
  api
          Execute bulk requests using JSON input
  schema
          Display schemas that Ludusavi uses
  gui
          Open the GUI
  help
          Print this message or the help of the given subcommand(s)

Options:
      --config <DIRECTORY>
          Use configuration found in a specific directory. It will be created if it does not exist
      --no-manifest-update
          Disable automatic/implicit manifest update checks
      --try-manifest-update
          Ignore any errors during automatic/implicit manifest update checks
      --debug
          Use max log level and open log folder after running. This will create a separate
          `ludusavi_debug.log` file, without any rotation or maximum size. Be mindful that the file
          size may increase rapidly during a full scan
  -h, --help
          Print help
  -V, --version
          Print version
```

## `backup --help`
```
Back up data

This command automatically updates the manifest if necessary.

Usage: ludusavi.exe backup [OPTIONS] [GAMES]...

Arguments:
  [GAMES]...
          Only back up these specific games. Alternatively supports stdin (one value per line)

Options:
      --preview
          List out what would be included, but don't actually perform the operation

      --path <PATH>
          Directory in which to store the backup. It will be created if it does not already exist.
          When not specified, this defers to the config file

      --force
          Don't ask for confirmation

      --no-force-cloud-conflict
          Even if the `--force` option has been specified, ask how to resolve any cloud conflict
          rather than ignoring it and continuing silently

      --wine-prefix <WINE_PREFIX>
          Extra Wine/Proton prefix to check for saves. This should be a folder with an immediate
          child folder named "drive_c" (or another letter)

      --api
          Print information to stdout in machine-readable JSON. This replaces the default,
          human-readable output

      --gui
          Use GUI dialogs for prompts and some information

      --sort <SORT>
          Sort the game list by different criteria. When not specified, this defers to the config
          file

          [possible values: name, name-rev, size, size-rev, status, status-rev]

      --format <FORMAT>
          Format in which to store new backups. When not specified, this defers to the config file

          [possible values: simple, zip]

      --compression <COMPRESSION>
          Compression method to use for new zip backups. When not specified, this defers to the
          config file

          [possible values: none, deflate, bzip2, zstd]

      --compression-level <COMPRESSION_LEVEL>
          Compression level to use for new zip backups. When not specified, this defers to the
          config file. Valid ranges: 1 to 9 for deflate/bzip2, -7 to 22 for zstd

      --full-limit <FULL_LIMIT>
          Maximum number of full backups to retain per game. Must be between 1 and 255 (inclusive).
          When not specified, this defers to the config file

      --differential-limit <DIFFERENTIAL_LIMIT>
          Maximum number of differential backups to retain per full backup. Must be between 0 and
          255 (inclusive). When not specified, this defers to the config file

      --cloud-sync
          Upload any changes to the cloud when the backup is complete. If the local and cloud
          backups are not in sync to begin with, then nothing will be uploaded. This has no effect
          on previews. When not specified, this defers to the config file

      --no-cloud-sync
          Don't perform any cloud checks or synchronization. When not specified, this defers to the
          config file

      --dump-registry
          Include the serialized registry content in the output. Only includes the native Windows
          registry, not Wine

      --include-disabled
          By default, disabled games are skipped unless you name them explicitly. You can use this
          option to include all disabled games

      --ask-downgrade
          Ask what to do when a game's backup is newer than the live data. Currently, this only
          considers file-based saves, not the Windows registry. This option ignores `--force`.

          You might want to use this if you force a backup on game exit, but you sometimes restore
          an older save temporarily to check something, and you don't want to accidentally back up
          that old save again. (If the save file gets updated during play, it will be considered
          newer.)

  -h, --help
          Print help (see a summary with '-h')
```

## `restore --help`
```
Restore data

Usage: ludusavi.exe restore [OPTIONS] [GAMES]...

Arguments:
  [GAMES]...
          Only restore these specific games. Alternatively supports stdin (one value per line)

Options:
      --preview
          List out what would be included, but don't actually perform the operation

      --path <PATH>
          Directory containing a Ludusavi backup. When not specified, this defers to the config file

      --force
          Don't ask for confirmation

      --no-force-cloud-conflict
          Even if the `--force` option has been specified, ask how to resolve any cloud conflict
          rather than ignoring it and continuing silently

      --api
          Print information to stdout in machine-readable JSON. This replaces the default,
          human-readable output

      --gui
          Use GUI dialogs for prompts and some information

      --sort <SORT>
          Sort the game list by different criteria. When not specified, this defers to Ludusavi's
          config file

          [possible values: name, name-rev, size, size-rev, status, status-rev]

      --backup <BACKUP>
          Restore a specific backup, using an ID returned by the `backups` command. This is only
          valid when restoring a single game

      --cloud-sync
          Warn if the local and cloud backups are out of sync. The restore will still proceed
          regardless. This has no effect on previews. When not specified, this defers to the config
          file

      --no-cloud-sync
          Don't perform any cloud checks or synchronization. When not specified, this defers to the
          config file

      --dump-registry
          Include the serialized registry content in the output. Only includes the native Windows
          registry, not Wine

      --include-disabled
          By default, disabled games are skipped unless you name them explicitly. You can use this
          option to include all disabled games

      --ask-downgrade
          Ask what to do when a game's backup is older than the live data. Currently, this only
          considers file-based saves, not the Windows registry. This option ignores `--force`.

          You might want to use this if you force a restore on game launch, but you don't always
          back up on game exit, so you might end up restoring an outdated backup by accident.

  -h, --help
          Print help (see a summary with '-h')
```

## `complete --help`
```
Generate shell completion scripts

Usage: ludusavi.exe complete <COMMAND>

Commands:
  bash
          Completions for Bash
  fish
          Completions for Fish
  zsh
          Completions for Zsh
  powershell
          Completions for PowerShell
  elvish
          Completions for Elvish
  help
          Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `backups --help`
```
Show backups

Usage: ludusavi.exe backups [OPTIONS] [GAMES]... [COMMAND]

Commands:
  edit
          Edit a backup
  help
          Print this message or the help of the given subcommand(s)

Arguments:
  [GAMES]...
          Only report these specific games. Alternatively supports stdin (one value per line)

Options:
      --path <PATH>
          Directory in which to find backups. When unset, this defaults to the restore path from the
          config file
      --api
          Print information to stdout in machine-readable JSON. This replaces the default,
          human-readable output
  -h, --help
          Print help
```

## `find --help`
```
Find game titles

Precedence: Steam ID -> GOG ID -> Lutris ID -> exact names -> normalized names. Once a match is
found for one of these options, Ludusavi will stop looking and return that match, unless you set
`--multiple`, in which case, the results will be sorted by how well they match.

If there are no matches, Ludusavi will exit with an error. Depending on the options chosen, there
may be multiple matches, but the default is a single match.

Aliases will be resolved to the target title.

This command automatically updates the manifest if necessary.

Usage: ludusavi.exe find [OPTIONS] [NAMES]...

Arguments:
  [NAMES]...
          Look up game by an exact title. With multiple values, they will be checked in the order
          given. Alternatively supports stdin (one value per line)

Options:
      --api
          Print information to stdout in machine-readable JSON. This replaces the default,
          human-readable output

      --multiple
          Keep looking for all potential matches, instead of stopping at the first match

      --path <PATH>
          Directory in which to find backups. When unset, this defaults to the restore path from the
          config file

      --backup
          Ensure the game is recognized in a backup context

      --restore
          Ensure the game is recognized in a restore context

      --steam-id <STEAM_ID>
          Look up game by a Steam ID

      --gog-id <GOG_ID>
          Look up game by a GOG ID

      --lutris-id <LUTRIS_ID>
          Look up game by a Lutris slug

      --normalized
          Look up game by an approximation of the title. Ignores capitalization, "edition" suffixes,
          year suffixes, and some special symbols. This may find multiple games for a single input

      --fuzzy
          Look up games with fuzzy matching. This may find multiple games for a single input

      --disabled
          Select games that are disabled

      --partial
          Select games that have some saves disabled

  -h, --help
          Print help (see a summary with '-h')
```

## `manifest --help`
```
Options for Ludusavi's data set

Usage: ludusavi.exe manifest <COMMAND>

Commands:
  show
          Print the content of the manifest, including any custom entries
  update
          Check for any manifest updates and download if available. By default, does nothing if the
          most recent check was within the last 24 hours
  help
          Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `cloud --help`
```
Cloud sync

Usage: ludusavi.exe cloud <COMMAND>

Commands:
  set
          Configure the cloud system to use
  upload
          Upload your local backups to the cloud, overwriting any existing cloud backups
  download
          Download your cloud backups, overwriting any existing local backups
  help
          Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `wrap --help`
```
Wrap restore/backup around game execution

Usage: ludusavi.exe wrap [OPTIONS] <--infer <LAUNCHER>|--name <NAME>> <COMMANDS>...

Arguments:
  <COMMANDS>...
          Commands to launch the game. Use `--` first to separate these from the `wrap` options;
          e.g., `ludusavi wrap --name foo -- foo.exe --windowed`

Options:
      --infer <LAUNCHER>
          Infer game name from commands based on launcher type [possible values: heroic, lutris,
          steam]
      --name <NAME>
          Directly set game name as known to Ludusavi
      --force
          Don't ask for any confirmation
      --force-backup
          Don't ask for confirmation when backing up
      --force-restore
          Don't ask for confirmation when restoring
      --no-force-cloud-conflict
          Even if another `--force` option has been specified, ask how to resolve any cloud conflict
          rather than ignoring it and continuing silently
      --gui
          Show a GUI notification during restore/backup
      --path <PATH>
          Directory in which to find/store backups. It will be created if it does not already exist.
          When not specified, this defers to the config file
      --format <FORMAT>
          Format in which to store new backups. When not specified, this defers to the config file
          [possible values: simple, zip]
      --compression <COMPRESSION>
          Compression method to use for new zip backups. When not specified, this defers to the
          config file [possible values: none, deflate, bzip2, zstd]
      --compression-level <COMPRESSION_LEVEL>
          Compression level to use for new zip backups. When not specified, this defers to the
          config file. Valid ranges: 1 to 9 for deflate/bzip2, -7 to 22 for zstd
      --full-limit <FULL_LIMIT>
          Maximum number of full backups to retain per game. Must be between 1 and 255 (inclusive).
          When not specified, this defers to the config file
      --differential-limit <DIFFERENTIAL_LIMIT>
          Maximum number of differential backups to retain per full backup. Must be between 0 and
          255 (inclusive). When not specified, this defers to the config file
      --cloud-sync
          Upload any changes to the cloud when the backup is complete. If the local and cloud
          backups are not in sync to begin with, then nothing will be uploaded. When not specified,
          this defers to the config file
      --no-cloud-sync
          Don't perform any cloud checks or synchronization. When not specified, this defers to the
          config file
      --ask-downgrade
          When restoring, ask what to do when a game's backup is older than the live data. When
          backing up, ask what to do when a game's backup is newer than the live data. Currently,
          this only considers file-based saves, not the Windows registry. This option ignores
          `--force`
  -h, --help
          Print help
```

## `api --help`
```
Execute bulk requests using JSON input.

If there is a problem with the entire input (e.g., malformed JSON or an invalid top-level setting),
then this will return a non-zero exit code. However, if the problem occurs while processing an
individual request, then the exit code will be zero, and the request's associated response will
indicate its error.

Some top-level errors, like an invalid CLI invocation, may result in a non-JSON output. However,
exit code zero will always use JSON output.

Use the `schema` command to see the input and output format.

Usage: ludusavi.exe api [INPUT]

Arguments:
  [INPUT]
          JSON data - may also be passed via stdin

Options:
  -h, --help
          Print help (see a summary with '-h')
```

## `schema --help`
```
Display schemas that Ludusavi uses

Usage: ludusavi.exe schema [OPTIONS] <COMMAND>

Commands:
  api-input
          Schema for `api` command input
  api-output
          Schema for `api` command output
  config
          Schema for config.yaml
  general-output
          Schema for general command output in --api mode (`backup`, `restore`, `backups`, `find`,
          `cloud upload`, `cloud download`)
  help
          Print this message or the help of the given subcommand(s)

Options:
      --format <FORMAT>
          [possible values: json, yaml]
  -h, --help
          Print help
```

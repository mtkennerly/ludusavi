# Command line
Ludusavi provides a [command line interface](https://en.wikipedia.org/wiki/Command-line_interface),
which you can use for automating tasks.

Run `ludusavi --help` for the overall CLI usage information,
or view info for specific subcommands, such as `ludusavi manifest update --help`.

You can also view the help text in [the CLI docs](/docs/cli.md).

## Demo
> ![CLI demo of previewing a backup](/docs/demo-cli.gif)

## JSON output
CLI mode defaults to a human-readable format, but you can switch to a
machine-readable JSON format with the `--api` flag.

Note that, in some error conditions, there may not be any JSON output,
so you should check if stdout was blank before trying to parse it.
If the command line input cannot be parsed, then the output will not be
in a stable format.

API output goes on stdout, but stderr may still be used for human-readable warnings/errors.
If stderr is not empty, you may want to log it,
since not all human-readable warnings have an API equivalent.

There is also an `api` command that supports using JSON for the input as well.

## Schemas
* [`--api` mode](/docs/schema/general-output.yaml)
* [`api` command input](/docs/schema/api-input.yaml)
* [`api` command output](/docs/schema/api-output.yaml)

## Example
Output for `backup --force --api`:

```json
{
  "errors": {
    "someGamesFailed": true,
  },
  "overall": {
    "totalGames": 2,
    "totalBytes": 150,
    "processedGames": 1,
    "processedBytes": 100,
  },
  "games": {
    "Game 1": {
      "decision": "Processed",
      "files": {
        "/games/game1/save.json": {
          "bytes": 100
        }
      },
      "registry": {
        "HKEY_CURRENT_USER/Software/Game1": {
          "failed": true
        }
      }
    },
    "Game 2": {
      "decision": "Ignored",
      "files": {
        "/games/game2/save.json": {
          "bytes": 50
        }
      },
      "registry": {}
    }
  }
}
```

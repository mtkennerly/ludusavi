# Custom games
You can create your own game save definitions on the `custom games` screen.
If the game name exactly matches a known game, then your custom entry will override it.

For file paths, you can click the browse button to quickly select a folder.
The path can be a file too, but the browse button only lets you choose
folders at this time. You can just type in the file name afterwards.
You can also use [globs]
(e.g., `C:/example/*.txt` selects all TXT files in that folder)
and the placeholders defined in the
[Ludusavi Manifest format](https://github.com/mtkennerly/ludusavi-manifest).
If you have a folder name that contains a special glob character,
you can escape it by wrapping it in brackets (e.g., `[` becomes `[[]`).

[globs]: https://en.wikipedia.org/wiki/Glob_(programming)

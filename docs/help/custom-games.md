# Custom games
You can create your own game save definitions on the `custom games` screen.
If the game name exactly matches a known game, then your custom entry will override it.

For file paths, you can click the browse button to quickly select a folder.
The path can be a file too, but the browse button only lets you choose
folders at this time. You can just type in the file name afterwards.
You can also use [globs](https://en.wikipedia.org/wiki/Glob_(programming))
(e.g., `C:/example/*.txt` selects all TXT files in that folder)
and the placeholders defined in the
[Ludusavi Manifest format](https://github.com/mtkennerly/ludusavi-manifest).
If you have a folder name that contains a special glob character,
you can escape it by wrapping it in brackets (e.g., `[` becomes `[[]`).

Installed names should be a bare folder name or relative path only (no absolute paths),
because Ludusavi will look for this folder in each root.
Ludusavi automatically looks for the game's own name as well,
so you only need to specify a custom folder name if it's different.
For example, if you have an other-type root at `C:\Games`,
and there's a game called `Some Game` installed at `C:\Games\sg`,
then you would set the installed name as `sg`.
If you had a bundled game like `C:\Games\trilogy\first-game`,
then you could set the installed name as `trilogy\first-game`.

# What if my saves aren't found?
Ludusavi mainly gets its data from [PCGamingWiki](https://www.pcgamingwiki.com).
The first step is to make sure that the game in question has an article,
and if so, that it has save and/or config locations in the `Game data` section
for your version of the game (e.g., Windows vs Linux, Steam vs Epic).

When the wiki has Windows save locations, but no Linux/Mac locations,
Ludusavi can *derive* some potential paths that aren't listed on the wiki,
such as checking Steam's `compatdata` folder for the game's app ID.
Sometimes, the fact that the game is running via Proton
or was loaded as a non-Steam game can affect the exact save location,
which may require adding/fixing the wiki data.

For games that have a PCGamingWiki article, but no save info,
Ludusavi also checks for Steam Cloud metadata as a fallback.
If the game doesn't have Steam Cloud support, then this won't apply,
and the Steam Cloud info is ignored once the wiki has save locations listed.

Every few hours, the latest changes from PCGamingWiki and Steam are assembled into the
[primary manifest](https://github.com/mtkennerly/ludusavi-manifest).
Ludusavi itself relies on this for save info,
rather than constantly checking the wiki when you run the application.
If the save location is listed on PCGamingWiki,
but was only added in the last few hours,
then it may simply not have made its way into the manifest yet.
You can also use Ludusavi's "other" screen to check when the manifest was last downloaded.

If the paths seem to be listed already, but Ludusavi still doesn't find it,
then try double checking your [configured roots](/docs/help/roots.md).
Ludusavi may only be able to scan some paths if an applicable root is configured.
For example, having a Steam root will enable Ludusavi to check its `compatdata` folder.

## Flatpak
If you're using Flatpak on Linux, then by default,
Ludusavi only has permission to view certain folders.
You can use a tool like Flatseal to grant access to additional folders.

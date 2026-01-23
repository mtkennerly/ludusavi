## Unreleased

* Added:
  * CLI: `wrap` now supports `--no-backup` and `--no-restore`.
  * For developers, the crates.io release now includes a library crate
    that you can use to access some of Ludusavi's internals.
    This is highly experimental and subject to change,
    but you're welcome to give it a try in its early stages.
* Fixed:
  * Native Linux saves were not detected in some Flatpak roots.
    ([Contributed by madscientist16](https://github.com/mtkennerly/ludusavi/pull/556))
  * For Lutris roots, some GOG and native Linux install folders were not properly detected.
  * For Heroic roots that point to a Flatpak installation,
    if Heroic's game installation directory contained `/home/user/games`,
    then Ludusavi would expect `/home/user/games` to actually exist,
    whereas Flatpak rules would cause Heroic to use `/home/user/.var/app/com.heroicgameslauncher.hgl/games` instead.
  * CLI: The `backup` command would sync games to the cloud
    if the scan found any new/changed files,
    even if it didn't create a new backup for the game.
    This now behaves like the GUI and only syncs games that are newly backed up.
  * GUI: Text can now be entered using input method editors.
* Changed:
  * The Mac release is now compiled on Mac OS 14 (ARM) instead of Mac OS 13 (Intel)
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/13046).

## v0.30.0 (2025-11-09)

* Added:
  * You can now configure game-specific Wine prefixes in a custom entry.
  * GUI: In the scan results, there is a button to copy registry key paths.
    On Windows, there is also a button to open the key in Regedit.
  * GUI: Backup comments may now contain multiple paragraphs.
  * CLI: `backups edit` command to update a backup's lock state and comment.
    The `api` command now also supports an `editBackup` request.
  * CLI: `config path` command to print the path to the active config file.
  * CLI: The `backup` and `restore` commands now have an `--include-disabled` option
    when you want to bulk process games that were disabled in the config.
  * CLI: Global `--debug` option to increase log level and open log folder after running.
    This is mainly to help users who are submitting bug reports.
  * CLI: The `backup`/`restore`/`wrap` commands now support an `--ask-downgrade` option.
    This is intended as a protection for cases such as when you launch games with `wrap` and `--force`,
    but if the backup didn't happen after your last session (e.g., your computer crashed),
    then on the next launch, an outdated backup would be restored.
  * CLI: `gui --custom-game Title` command to open a specific entry on the custom game screen.
* Fixed:
  * The cloud "synchronize automatically" setting did not work in GUI mode,
    even though it did work correctly in CLI mode.
    This issue was introduced in v0.26.0.
    Please use the upload icon on the "other" screen to ensure your existing backups are synchronized.
* Changed:
  * If the `WGPU_POWER_PREF` environment variable is not set,
    then Ludusavi will automatically set it to `high` while running.
    This has fixed application crashes on several users' systems,
    but is ultimately dependent on graphics hardware and drivers.
    If you experience any issues with this, please report it.
  * Updated translations, including partial support for Norwegian.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.29.1 (2025-04-17)

* Fixed:
  * Glob-based backup exclusions did not work correctly.
    (This issue was introduced in v0.29.0.)
* Changed:
  * The standalone Linux release is now compiled on Ubuntu 22.04 instead of Ubuntu 20.04
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/11101).

## v0.29.0 (2025-04-07)

* Added:
  * A custom game's installed name may now be set to a relative path with multiple folders,
    rather than only supporting a single bare folder name.
  * CLI: The `wrap` command now supports `--force-backup` and `--force-restore`
    for more granular control than `--force`.
  * GUI: During a scan, you can click on the progress bar to see a list
    of the games currently being scanned and how long each one is taking.
    This can be useful to identify why a scan might be taking longer than expected.
  * CLI: When backing up or restoring,
    if your local and cloud backups are in conflict,
    Ludusavi will now ask you if you'd like to resolve it by downloading or uploading.
    You can also choose to ignore the conflict (which is the existing behavior),
    and `--force` will automatically ignore any conflicts.
    You can combine `--force` and `--no-force-cloud-conflict`
    to be prompted only when there is a conflict.
  * CLI: When using `--gui` in the commands that support it,
    dialog titles now include the game's name (if you've specified only one)
    or the total number of games (if you've specified more than one).
* Fixed:
  * For home folder roots, Ludusavi skipped any paths containing `<storeUserId>`,
    on the assumption that it shouldn't be applicable to non-store-specific roots.
    However, there are some cases where it's worth scanning regardless,
    so Ludusavi will now use a wildcard match like it does for other root types.
  * On Windows, a backup would fail if the original file were encrypted
    and the backup destination could not be encrypted.
    Now, in this situation, the backup will proceed without encryption.
    ([Contributed by Summon528](https://github.com/mtkennerly/ludusavi/pull/476))
  * Ludusavi did not detect some save data for Heroic Epic games that had been uninstalled.
  * System folders and game installed names were not scanned properly if they contained `[` or `]`,
    because Ludusavi did not escape them before integrating them into larger glob patterns.

    For roots, there was a similar issue with escaped brackets (`[[]` or `[]]`).
    Although root paths do support globs,
    Ludusavi internally expands each configured root into one root per glob match,
    but it did not then escape each expanded root path before integrating it into a larger pattern.
  * GUI: In the scan results, some elements could get squished with long file paths.
  * CLI: In the scan results, if you enabled the option to skip backups when there are only removals,
    those games would still count towards the change tally.
  * CLI: In the default scan results output format,
    registry content would be dumped even without `--dump-registry`.
  * On Windows, some paths were unnecessarily scanned twice.
  * On Windows, some network share paths were not scanned properly in certain contexts.
  * When an Rclone command failed,
    the error message did not include quotes around arguments with spaces,
    even though the actual command did account for spaces.
* Changed:
  * When a disabled game is new or updated in the scan results,
    that game's change badge will now be faded,
    and it will be sorted with games that do not have changes.
  * GUI: In some cases, Ludusavi would automatically close any open modal
    in order to show a different one,
    which could be inconvenient if you were filling out fields in certain modals.
    Now, Ludusavi will redisplay the older modal when the new one is closed.
  * When Ludusavi checks your non-Steam games added as shortcuts in Steam,
    it now normalizes the titles to allow for more lenient matching.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.28.0 (2025-01-15)

* Added:
  * On Linux, for Lutris roots that point to a Flatpak installation,
    Ludusavi now checks `$XDG_DATA_HOME` and `$XDG_CONFIG_HOME`
    inside of the Flatpak installation of Lutris.
  * Custom games now let you specify installed folder names.
    This can be used to satisfy the `<base>` and `<game>` path placeholders
    in cases where Ludusavi can't automatically detect the right folder.
    For more info, [see the custom games document](/docs/help/custom-games.md).
  * On the "other" screen,
    there is a new option to skip backups when saves are only removed but not added/updated.
    This can be useful because uninstalling a game may cause some of its data (but not all) to be removed,
    but you may not want to exclude that data from your backups yet.
  * CLI: `config show` command.
  * CLI: The `backup`, `restore`, `cloud upload`, and `cloud download` commands
    now support a `--gui` option for graphical dialog prompts.
  * CLI: The `backup` and `restore` commands now support a `--dump-registry` option,
    which includes the serialized registry content in the output.
    This may be useful if you're consuming the `--api` output to back up with another tool,
    but don't have a good way to check the registry keys directly.
  * CLI: The `find` command now supports `--fuzzy` and `--multiple` options.
    This is also available for the `api` command's `findTitle` request.
  * CLI: The `wrap` command now supports several options from the `backup` command:
    `--path`,
    `--format`,
    `--compression`,
    `--compression-level`,
    `--full-limit`,
    `--differential-limit`,
    `--cloud-sync`,
    `--no-cloud-sync`.
* Changed:
  * When the game list is filtered,
    the summary line (e.g., "1 of 10 games") now reflects the filtered totals.
  * The `enable/disable all` buttons are now constrained by the active filter.
  * GUI: Changed some icons to a softer version.
  * CLI: When using the `--gui` option of any command that supports it,
    errors at the end of the process will also be reported via dialogs.
    This does not apply to CLI parse errors.
  * Application crash and CLI parse errors are now logged.
  * Updated translations, including partial support for Vietnamese and Swedish.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * If a custom game's title begins or ends with a space,
    that custom game will now be ignored.
    Previously, Ludusavi would make a backup folder for the game including the space,
    but the OS (namely Windows) would remove the space from the folder title,
    causing unpredictable behavior when Ludusavi couldn't find the expected folder name.
  * GUI: In backup mode, if Ludusavi failed to prepare the backup target folder,
    it would get stuck where you couldn't cancel/restart the operation.
  * CLI: `find --normalized` now better prioritizes the closest match
    when multiple manifest entries have the same normalized title.
  * Some default paths are now formatted more consistently.
  * GUI: There was an error when the backup/restore paths were relative to the working directory.
  * When backing up a read-only file using the simple format,
    Ludusavi would fail to set the backed up file's modified time.

## v0.27.0 (2024-11-19)

* Added:
  * Support for installing via [cargo-binstall](https://github.com/cargo-bins/cargo-binstall).
* Changed:
  * Windows registry backups are now saved as `*.reg` files instead of `*.yaml`.
    Existing backups will not be affected.
  * On Linux, Ludusavi previously reported its application ID as just `ludusavi`,
    which meant the desktop file should be named `ludusavi.desktop` to show the right icon.
    However, that name does not follow the Freedesktop.org `desktop-entry` specification.

    To better conform, Ludusavi now reports its ID as `com.mtkennerly.ludusavi`
    (except for Flatpak, which will use `com.github.mtkennerly.ludusavi` for legacy reasons).
    If you need to preserve the original behavior,
    you can set `LUDUSAVI_LINUX_APP_ID=ludusavi` in your environment variables.

    ([Prototyped by OlegAckbar](https://github.com/mtkennerly/ludusavi/pull/417))
  * Dialogs (folder picker and `wrap --gui` prompts) now use GTK on Linux.
    The previous system relied on Zenity/KDialog,
    which could behave poorly depending on the version or in a Flatpak context.
  * The standalone Mac release is now compiled on Mac OS 13 instead of Mac OS 12
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/10721).
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * The registry format change also resolved an issue where very large (over 100 MB)
    `registry.yaml` files could be slow to read and consume a lot of extra memory,
    whereas the same data in `.reg` format can be loaded without issue.
  * When set to only keep 1 full backup and 0 differential backups using the simple format,
    Ludusavi keeps the existing backup in place and just adds/removes any changed files.
    However, after removing obsolete files, Ludusavi could leave empty directories behind.
    Now, Ludusavi will clean these up as well after creating a new backup for a game.
  * GUI: After a backup, if a file were removed,
    its change status wouldn't immediately refresh.
  * GUI: When performing a multi-game scan with a filter active,
    the visible games would be backed up or restored even if they were disabled.
  * GUI: When performing a multi-game scan on the restore screen with a filter active,
    the scan would exclude games that were disabled for backup rather than disabled for restore.
  * Ludusavi would try to scan games (custom or from secondary manifest) with a blank title.
    In the GUI, they would be omitted from the results,
    while on the CLI, they would be reported without a title.
    Now such games are ignored when scanning.

## v0.26.0 (2024-10-29)

The Linux and Mac downloads are now provided in `.tar.gz` format
to better preserve the files' executable permissions.

* Added:
  * Paths may now use the `<storeGameId>` placeholder.
    This is supported in Steam, GOG, and Lutris roots.
    For Steam roots, this also supports shortcuts to non-Steam games,
    where the placeholder will map to the shortcut's dynamic app ID.
  * Paths may now use the `<winLocalAppDataLow>` placeholder.
  * GUI: On the backup and restore screens,
    if you activate the filter options,
    then the backup/restore buttons will only process the currently listed games.
    This allows you to quickly scan a specific subset of games.
  * You can now choose whether a custom game will override or extend
    a manifest entry with the same name.
    Previously, it would always override the manifest entry completely.
  * GUI: Custom games can now be expanded/collapsed, sorted, and filtered.
  * GUI: Custom games now have an icon to indicate when they override/extend a manifest entry.
  * You can now configure redirects to be processed in reverse sequence when restoring.
  * GUI: On the custom games screen,
    when you use the button to preview a custom game,
    the window will switch to the backup screen and show you the results for that game.
  * GUI: There is now a button to quickly reset the game list filters,
    while still leaving the filter options open.
* Fixed:
  * Files on Windows network shares were not backed up correctly.
    For example, a file identified as `\\localhost\share\test.txt`
    would be backed up as `<game>/drive-____UNC_localhost_share_test.txt`
    instead of the intended `<game>/drive-____UNC_localhost_share/test.txt`.
  * When Steam was not installed, the logs would contain a `warning`-level message.
    This has been demoted to an `info`-level message.
  * GUI: Fixed some inconsistent spacing between elements.
  * CLI: On Linux, the `wrap` command's `--infer steam` option would fail
    to find the `SteamAppId` environment variable due to a case mismatch.
  * CLI: In some error conditions, the `wrap` command would show an alert
    and wait for the user to press a key, even if `--force` was specified.
    Now, with `--force`, Ludusavi will not wait for any input.
  * Old log files were not deleted when stored on a Windows network share.
  * GUI: The title filter was case-sensitive.
* Changed:
  * GUI: After successfully backing up or restoring a game,
    the status icons (new/updated/etc) will be cleared for that game.
  * GUI: If the GUI fails to load, Ludusavi will try to log the error info.
  * GUI: When you launch Ludusavi, the window now ensures that it gains focus.
  * GUI: Modals now display on top of the app with a transparent background.
  * GUI: On the backup and restore screens,
    the filter controls now wrap depending on the window size.
  * GUI: The backup format and retention settings are now on the "other" screen,
    instead of being accessed via the gear icon on the backup screen.
  * GUI: Some uses of "select"/"deselect" have been changed to "enable"/"disable".
  * GUI: The game list filters now have a different background color.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.25.0 (2024-08-18)

* Added:
  * You can now ignore specific manifests during scans.
    For example, if you only want to back up custom games,
    you can now disable the primary manifest's entries.
  * GUI: On startup and once every 24 hours,
    Ludusavi will check if a new version is available and notify you.
  * GUI: When left open,
    Ludusavi will automatically check for manifest updates once every 24 hours.
    Previously, this check only occurred when the app started.
  * Manifests may now include a `notes` field.
    If a game has notes in the manifest,
    then the backup screen will show an info icon next to the game,
    and you can click the icon to display the notes.
    The primary manifest does not (yet) contain any notes,
    so this mainly applies to secondary manifest authors.
  * GUI: You can now filter scan results by which secondary manifest defined each game.
    You can also filter to display custom games only.
  * CLI: The `api` command now supports a `checkAppUpdate` message.
  * Linux: Added keywords to the `.desktop` file.
    ([Contributed by Merrit](https://github.com/mtkennerly/ludusavi/pull/377))
* Fixed:
  * CLI: Some commands would fail with relative path arguments.
* Changed:
  * In the config file, `manifest.url` is now set to `null` by default
    to indicate that the default URL should be used,
    rather than explicitly putting the default URL in the file.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.24.3 (2024-07-01)

* Fixed:
  * If two consecutive differential backups both ignored *different* save files
    *and* none of those files were ignored in the associated full backup,
    then the second differential backup would fail to redeclare
    the first differential backup's ignored saves.
  * If you redirected all of the saves for a game that already had a backup,
    then the next scan would list the game as new instead of updated.
  * GUI: On Mac, the file/folder selector would cause the app to crash.

## v0.24.2 (2024-06-28)

* Fixed:
  * When multi-backup was enabled and Ludusavi backed up a game for the first time,
    it would first insert an empty backup in that game's `mapping.yaml`
    and then insert the real backup after.
    This behavior was meant for updating old backups from before multi-backup was added,
    but it was mistakenly being applied to brand new backups as well.

    Ludusavi will automatically detect and fix this.
    If the empty backup has a differential backup associated,
    then the oldest differential backup will replace the empty full backup.
    Otherwise, Ludusavi will remove the entry for the empty backup.

    **If you use Ludusavi's cloud sync feature,**
    please run a preview in restore mode,
    which will automatically fix any of these incorrect initial backups,
    and then perform a full cloud upload on the "other" screen.
  * For Lutris roots, after reading `pga.db`,
    Ludusavi did not properly combine that data with the data from the `games/*.yml` files.
    ([Verified by nihaals](https://github.com/mtkennerly/ludusavi/pull/359))
  * Ludusavi assumed that a Lutris root would contain both `games/` and `pga.db` together.
    That's true for new installations of Lutris,
    but older/existing installations would store them separately
    (e.g., `~/.config/lutris/games` and `~/.local/share/lutris/pga.db`).
    To fix this, you can now specify a different `pga.db` path explicitly.
    In some cases, Ludusavi can prompt you to update the root automatically.
  * CLI: The `find` command's `--steam-id` and `--gog-id` options
    only considered primary IDs from the manifest.
    They will now also consider secondary IDs (e.g., for DLC or different editions).
* Changed:
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.24.1 (2024-06-15)

* Fixed:
  * Symlinks were incorrectly traversed when applying redirects.
    For example, if you had a backup-type redirect from `/old` to `/new`,
    but `/new` happened to be a symlink to `/newer` on your system,
    then the backup would incorrectly contain a reference to `/newer`.
  * Redirects could match a partial folder/file name.
    For example, a restore-type redirect from `C:/old` to `C:/new`
    would *also* redirect `C:/older` to `C:/newer` (`C:/[old -> new]er`).
  * On Linux, if a file name contained a colon (`:`),
    it would fail to back up.
  * GUI: When using a game's context menu to create a custom entry,
    Ludusavi did not scroll down to the new entry.
* Changed:
  * Updated translations, including a new translation for Finnish.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.24.0 (2024-06-08)

* Added:
  * On the "other" screen,
    you can choose not to back up games with cloud support on certain stores.
    If a game is customized or already has local backups,
    then Ludusavi will continue backing it up regardless.
  * For Heroic roots, Ludusavi now supports Amazon and sideloaded games.
  * For Lutris roots,
    Ludusavi now scans `pga.db` in addition to `games/*.yml`
    in case the YAML files do not contain all of the necessary information.
  * CLI: There is a new `api` command that can be used for bulk queries.
    Right now, it only supports looking up titles (analogous to the `find` command).
  * CLI: There is a new `schema` command to display some of Ludusavi's schemas.
  * CLI: The `find` command now accepts a `--lutris-id` option.
  * CLI: The `backups` command output now includes each game's backup directory.
* Changed:
  * Title normalization now ignores apostrophes and quotation marks
    (e.g., `ludusavi find --normalized "Mirrors Edge"` will find `Mirror's Edge`).
  * Some additional fields in the config file have been made optional.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * For Heroic and Lutris roots,
    if you had multiple copies of the same game,
    Ludusavi would only use the metadata from one of them.
  * GUI: The game-level duplication badge did not always fade out when the conflicts were resolved.

## v0.23.0 (2024-04-27)

* Added:
  * CLI: The `wrap` command now supports some new arguments:
    `--infer steam`, `--infer lutris`, and `--force` to skip confirmations
  * GUI: File sizes are now displayed for each file and directory.
    ([Contributed by JackSpagnoli](https://github.com/mtkennerly/ludusavi/pull/308))
  * When a save fails to back up or restore, you can now see a specific error message per save.
    Previously, this was only available in the log file.
    For the GUI, you can hover over the "failed" badge to view the error.
    Note that these errors are shown as-is for troubleshooting and may not be translated.
  * You can now set aliases to display instead of the original name.
    This does not affect the CLI when using `--api`.
  * On Linux, for Steam roots that point to a Flatpak installation,
    Ludusavi now checks `$XDG_DATA_HOME` and `$XDG_CONFIG_HOME`
    inside of the Flatpak installation of Steam.
  * Updated translations, including new in-progress translations for Traditional Chinese and Turkish.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * Ludusavi would crash when reading a non-UTF-8 mapping.yaml file.
    This wouldn't normally happen, but could arise from external modifications.
  * GUI: On Linux with Wayland, the app ID property was not being set,
    which caused some issues like not showing the window icon and not grouping the window properly.
    ([Contributed by ReillyBrogan](https://github.com/mtkennerly/ludusavi/pull/334))
  * CLI: During slow processing (e.g., cloud upload or a game with huge saves),
    the progress bar timer wouldn't update.
  * GUI: After performing a cloud upload preview on the other screen,
    the very next backup preview wouldn't do anything.
  * GUI: You can now use undo/redo shortcuts for backup comments.
  * CLI: The `wrap` command did not fail gracefully when the game launch commands were missing.
  * CLI: Several commands did not resolve aliases.
  * CLI: The `cloud` commands did not reject unknown game titles.
  * If a game had more data that failed to back up than succeeded,
    then the backup size would be reported incorrectly.
* Changed:
  * The way Ludusavi parses file paths internally has been overhauled.
    The majority of the observable behavior is the same,
    but it is now more predictable and correct when parsing Linux-style paths on Windows and vice versa.

    Some behavioral changes worth noting:

    * You can now configure redirects that change Windows/Linux-style paths into the other format.
      For example, if you configure a backup redirect from `C:\games` to `/opt/games`,
      then the backup will contain references to `/opt/games`.
      (Previously, `/opt/games` would turn into `C:/opt/games` when parsed on Windows,
      and `C:\games` would turn into `./C_/games` when parsed on Linux.)
    * On Windows, you can no longer write `/games` as an alias of `C:\games`.
      These are now treated as distinct paths.
      (Previously, on Windows, Linux-style paths were interpreted as `C:` paths.)
    * If you try to restore Windows-style paths on Linux or vice versa,
      it will now produce an error,
      unless you've configured an applicable redirect.
  * GUI: On Windows, the way Ludusavi hides its console in GUI mode has changed,
    in order to avoid a new false positive from Windows Defender.

    Instead of relaunching itself, Ludusavi now detaches the console from the current instance.
    This reverts a change from v0.18.1,
    but care has been taken to address the problems that originally led to that change.
    If you do notice any issues related to this, please report them.
  * GUI: Previously, when you changed settings, Ludusavi would save each change immediately.
    It now waits for 1 second in case there is another change,
    so that typing and other fast, successive edits are batched.
  * CLI: Previously, the `restore` and `backups` (not `backup`) commands would return an error
    if you specified a game that did not have any backups available to restore.
    This was inconsistent with the `backup` command,
    which would simply return empty data if there was nothing to back up.
    Now, `restore` and `backups` will also return empty data if there are no backups.
  * CLI: Some deprecated flags have been removed from the `backup` command:
    `--merge`, `--no-merge`, `--update`, and `--try-update`.
  * When synchronizing to the cloud after a backup,
    Ludusavi now instructs Rclone to only check paths for games with updated saves.
    This improves the cloud sync performance.
  * The following are now configured as default arguments for Rclone:
    `--fast-list --ignore-checksum`.
    These should improve performance in most cases.
    You can change or remove these on the "other" screen.
  * GUI: During a backup or restore,
    if the "synchronize automatically" cloud setting is enabled,
    then the progress bar will display "cloud" instead of "scan" during the cloud operations.
  * Differential backup names now end with "-diff".
    This does not affect existing backups.

## v0.22.0 (2023-12-26)

* Added:
  * You can now configure additional manifests,
    which Ludusavi will download and use just like the primary one.
    This allows the community to create additional save lists for specific purposes
    that might not be covered by PCGamingWiki.
  * You can now configure a custom game as an alias for another game,
    without having to make a copy of the other game's info.
    On the custom games screen,
    use the dropdown to toggle between "game" (default) and "alias".
  * You can now configure roots for OS installations on other drives.
    New root types: `Windows drive`, `Linux drive`, `Mac drive`
  * Ludusavi can now scan Legendary games on their own without Heroic.
    New root type: `Legendary`
  * CLI: `wrap` command to do a restore before playing a game and a backup afterwards.
    ([Contributed by sluedecke](https://github.com/mtkennerly/ludusavi/pull/235))
  * When a path or URL fails to open, additional information is now logged.
  * On Windows, Ludusavi can now back up additional types of registry data:
    `REG_NONE`,
    `REG_DWORD_BIG_ENDIAN`,
    `REG_LINK`,
    `REG_RESOURCE_LIST`,
    `REG_FULL_RESOURCE_DESCRIPTOR`,
    `REG_RESOURCE_REQUIREMENTS_LIST`.
  * On Windows, Ludusavi now recognizes if you've moved the `%USERPROFILE%\Saved Games` folder.
* Changed:
  * GUI: A different icon is now used for the button to hide the backup comment field.
    The previous icon (a red X) could have been misinterpreted as "delete" rather than "close".
  * GUI: When you click the filter icon on the backup/restore screen,
    the title search field is automatically focused.
  * CLI: Help text is now styled a bit differently.
  * Updated translations, including a new in-progress Czech translation.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * GUI: On some systems using Wayland, Ludusavi would crash on startup.
  * When storing file modified times in zip archives,
    if the year is too old for zip to support (i.e., before 1980),
    Ludusavi will now round up to the earliest supported date (1980-01-01).
  * When backing up a malformed `dword`-type value from the registry,
    Ludusavi would silently convert it to a default 0,
    which could result in data loss when restored.
    Now, invalid registry values are backed up and restored as-is.
  * If Ludusavi encountered an error when restoring a specific file,
    it would retry up to 99 times in case it was just a temporary error.
    This was primarily intended to handle cases of duplicate backups that might cause a file to be busy,
    but it would also cause excessive delays for other, persistent errors.
    Now, Ludusavi will only try once per file.
  * GUI: When a custom game was disabled, its refresh button would do nothing.
    The refresh button will now be disabled for that game.

## v0.21.0 (2023-08-22)

* Added:
  * GUI: Thanks to updates in [Iced](https://github.com/iced-rs/iced),
    there is now much better support for non-ASCII characters.
    This means that several translations are now properly supported:
    Simplified Chinese, Japanese, Korean, and Thai.
    Unfortunately, there are still technical limitations with Arabic,
    so that translation remains experimental via the config file.
  * GUI: For custom games in scan results,
    you can click on the "custom" badge to jump to the corresponding entry.
* Changed:
  * GUI: Rendering now uses DirectX/Vulkan/Metal instead of OpenGL.
    For systems that don't support those, there is a fallback software renderer as well.
  * GUI: Ludusavi now bundles and uses the Noto Sans font for consistency,
    but some languages will still depend on your system fonts.
* Fixed:
  * If an invalid manifest file were downloaded, Ludusavi would correctly show an error,
    but then after relaunching, it would get stuck on an "updating manifest" screen.
  * On Linux, if Ludusavi were installed via Flatpak, then `XDG_CONFIG_HOME` and `XDG_DATA_HOME`
    would be set inside of the Flatpak environment, preventing it from finding some saves.
    Now, Ludusavi will also check the default paths (`~/.config` and `~/.local/share` respectively).
  * For Heroic roots, Ludusavi now also checks the `legendaryConfig` folder used by Heroic 1.9.0.
  * Saves associated with the Ubisoft Game Launcher folder were not detected
    on Linux when installed with Steam and Proton.
  * On non-Windows systems, when recursively finding files in a directory,
    file/folder names containing a backslash would cause an error.
    For now, these files will be ignored until they are properly supported.
  * When using shift+click on a path selector icon to browse the path,
    it will now handle some manifest `<placeholder>`s.
  * In paths, `<storeUserId>` next to `*` would trigger an error.
  * GUI: When switching screens and then expanding a section,
    the scroll position did not remain visually stable.

## v0.20.0 (2023-07-10)

* Added:
  * The restore screen now supports deselecting individual saves
    (like you already could on the backup screen).
  * You can now use glob syntax for file paths in the "backup exclusions" section.
  * CLI: Commands that take a list of games now support reading stdin (one game per line).
    For example, `ludusavi find --steam-id 504230 | ludusavi backup --preview`.
  * CLI: The `find` command will now report multiple results if you don't specify a name or ID.
    The command also has new options for filtering these results: `--disabled` and `--partial`.
    For example, `ludusavi find --restore --disabled` will list all games that can be restored and are disabled.
  * Support for checking secondary/associated Steam IDs for a game.
    This is mainly useful for discovering Proton prefix folders of DLC,
    since DLC saves may be kept separately from base game saves.
    Specifically, this detection is based on the `steamExtra` field from the manifest.
  * A "custom" badge is shown next to custom games in scan results.
  * Option to filter scan results by change status (new/updated/unchanged/unscanned).
    ([Contributed by kekonn](https://github.com/mtkennerly/ludusavi/pull/226))
  * For buttons that open a path selector dialog,
    shift+click will open the configured path in your file explorer.
* Fixed:
  * When restoring registry saves,
    multi-string values would be restored as expandable string values,
    and expandable string values would be restored as multi-string values.
    This only affected the restore process; backups would still be correct.
    This issue was introduced in v0.18.0.
  * For Lutris roots, the `<base>` placeholder was resolved generically
    instead of using the Lutris-specific logic.
  * For Lutris roots, when inferring the `<base>` from the `exe` field,
    Ludusavi assumed that the path would be absolute, but it could also be relative.
    Now, Ludusavi will combine the `prefix` and `exe` fields if necessary.
* Changed:
  * All path selectors now use the same icon.
  * The button to find missing roots now uses a search icon instead of a refresh icon.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.19.0 (2023-06-02)

* Added:
  * On the restore screen, there is a "validate" button to check whether
    your backups are missing any files declared in their mapping.yaml.
    This is intended to help rectify a bug identified below.
  * Automatic detection of non-Flatpak Lutris roots (`~/.config/lutris`).
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

    A partial translation for Thai has been added, but it only has experimental support
    because of a [technical limitation](https://github.com/mtkennerly/ludusavi/issues/9).
    You can enable it by editing the config file directly with language code `th-TH`,
* Fixed:
  * If you had configured a backup-only or bidirectional redirect
    and you were using simple backups,
    then the first backup for a game would complete successfully,
    but a subsequent backup would fail because Ludusavi would mark the redirect target
    as a removed file.
  * If you had configured a backup-only or bidirectional redirect
    and you were using zip-based backups,
    then the redirected files would not be included in the backup.

    You can check if this affects you by going to the restore screen and clicking the "validate" button.
    If it finds any issues, it will prompt you to make new full backups for the games in question.
  * Compatibility with Heroic 2.7.0+, which now uses `store_cache/gog_library.json` instead of `gog_store/library.json`.
  * For Lutris, the `game_slug` field is no longer required,
    since Ludusavi only uses it for logging when available.
  * The Spanish and Russian translations were set incorrectly in the config file.
    If you selected Spanish, it would display normally, but the config file would be set to Russian.
    If you selected Russian and restarted the app, it would display in Japanese.

## v0.18.2 (2023-05-21)

* Fixed:
  * When a Lutris game file does not include the `game > working_dir` field,
    Ludusavi will now try to fall back to the `game > exe` field and cut off the file name.
    Ludusavi will also log a more specific message when an expected field is missing.

## v0.18.1 (2023-05-21)

* Fixed:
  * Cloud backups would fail if the cloud path contained a backslash (`\`).
  * On Windows, if the default terminal application was the [Windows Terminal](https://aka.ms/terminal)
    (as opposed to the older Windows Console Host), then a couple of problems would happen
    when Ludusavi was launched from Windows Explorer:

    * An empty console window would stay open along with the GUI.
    * Asynchronous Rclone commands would fail.

    This was ultimately related to how Ludusavi hides the console in GUI mode.
    Now, instead of removing the console from the currently running instance,
    Ludusavi simply relaunches itself in a detached state.

## v0.18.0 (2023-05-20)

* Added:
  * You can now upload backups to the cloud.
    This integrates with [Rclone](https://rclone.org), so you can use any cloud system that it supports,
    and Ludusavi can help you configure some of the more common ones:
    Google Drive, OneDrive, Dropbox, Box, FTP servers, SMB servers, and WebDAV servers.

    For the GUI, refer to the "cloud" section on the "other" screen.
    For the CLI, use the `cloud` command group (e.g., `ludusavi cloud upload`).
  * The Lutris launcher is now supported as a root type.
    Ludusavi can find saves from Wine prefixes configured in Lutris.
  * The EA app is now supported as a root type.
  * On the restore screen, you can lock a backup
    so that it is kept indefinitely regardless of your retention settings.
  * Progress bars now show additional information (operation label, elapsed time, exact progress count).
  * Backups now record the operating system on which they were created.
    For the GUI, this is shown as a badge on the restore screen if you select a non-native backup.
    For the CLI, this is included in the output of the `backups` command.
  * Ludusavi now supports Flatpak IDs (if present in the manifest)
    in order to infer the correct `XDG_DATA_HOME` and `XDG_CONFIG_HOME`.
    At this time, the primary manifest does not specify Flatpak IDs for any games,
    but any such additions can be supported transparently in the future.
  * GUI: You can use (shift+)tab to cycle through text fields.
  * GUI: Input fields show an error icon for paths like `http://` and `ssh://`
    since these are not supported and will be mangled into a local path.
  * CLI: A standalone `manifest update` command.
* Changed:
  * The "merge" option has been removed, and merging is now always enforced.
    This option made sense before Ludusavi supported differential and cloud backups,
    but there was not much reason to turn off merging anymore.

    The CLI `backup` command's `--merge`/`--no-merge` flags are now ignored and will be removed in a future release.
  * CLI: The `backup` command's `--update`/`--try-update` flags are deprecated and will be removed in a future release.
    It was confusing because Ludusavi could still update the manifest without either flag,
    and other commands would also update the manifest but without equivalent flags to adjust the behavior.

    To simplify this and for consistency with the GUI, now the CLI will update the manifest automatically by default.
    To disable this, use the new `--no-manifest-update` global flag, which works across commands.
    To ignore errors in the update, use the new `--try-manifest-update` global flag.
  * CLI: The deprecated `--by-steam-id` option has been removed from the `backup`, `backups`, and `restore` commands.
    You can use the `find` command to replicate this functionality.
  * CLI: Using `--api` mode would silence some human-readable errors that would otherwise go on stderr.
    Since the API output itself goes on stdout, there's no harm leaving the other messages on stderr,
    so they are now allowed to print.
    In the future, these messages may be integrated into the API output directly.
  * GUI: If you try to close the program while an operation is ongoing,
    Ludusavi will cancel the operation cleanly first before closing.
    To skip the cleanup and force an immediate close (like in previous versions),
    you can simply try to quit a second time, but this isn't recommended if you can help it.
  * GUI: Subsections now have a slightly distinct background color to help tell them apart.
  * GUI: Adjusted some spacing/padding. A few more scanned games can fit on screen now.
  * Log files now include timestamps.
  * Some obsolete fields were removed from the config file.
    This won't have any effect on you unless you were using a version older than v0.14.0.
    If so, then just update to v0.17.1 first so that Ludusavi can migrate the affected settings.
    Fields: `manifest.etag`, `backup.recentGames`, `restore.recentGames`, and `restore.redirects`.
  * Some config fields weren't serialized if they matched the default value.
    Now they're serialized anyway in case the default value were to ever change.
    Fields: `backup.filter.excludeStoreScreenshots`, `scan.showDeselectedGames`, `scan.showUnchangedGames`, and `scan.showUnscannedGames`.
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Fixed:
  * Significantly improved performance of zip file extraction.
    Ludusavi had been unnecessarily reopening the zip for each file inside.
    In the most extreme case tested (40,864 files and 8.70 GB for a single game),
    the time was reduced from 12+ hours to 4 minutes.
  * In secondary manifests, relative paths (beginning with `./` and `../`) were not correctly resolved.
  * GUI: In some cases, the scroll position would be set incorrectly
    when changing screens or when closing a modal.
  * CLI: A bare `manifest` command was allowed, even though it did nothing.
    You can still use `manifest --help` for subcommand info.
  * The log message "ignoring unrecognized Heroic game" could be recorded incorrectly when doing partial scans.

## v0.17.1 (2023-04-10)

* Fixed:
  * GUI: As part of the thread configuration feature in v0.17.0,
    Ludusavi started defining a custom Tokio runtime initialization.
    However, this did not enable Tokio's IO and time features,
    resulting in a crash when attempting to display timed notifications.
  * GUI: When using the "customize" option from the scan list,
    the new custom game's fields were not filled in properly.
  * GUI: If a game was disabled,
    then the "back up" option in that game's "..." menu would not do anything.
  * GUI: If you scanned a few specific games in the list,
    but there were others in an unscanned state (recent games from a previous session),
    then the next full backup would only process the few games that were scanned.

## v0.17.0 (2023-04-09)

* Added:
  * A new "removed" status (icon: `x`) is now displayed for saves.
    This indicates that a save from the latest backup no longer exists on the system.
    If a game has some removed saves, then that game will be marked as updated and will trigger a new backup.
    If 100% of a game's saves are removed, then the game won't be listed, and no backup will be performed.
  * Support for secondary manifests bundled with games.
    If a game includes a `.ludusavi.yaml` file,
    then it will be incorporated into the backup scan.
  * Option to sort games by status: new -> different -> same -> unscanned.
    This is now the default sort order for new installations of Ludusavi.
  * Option to override the maximum threads used for scanning games in parallel.
    You can also override this via the `LUDUSAVI_THREADS` environment variable.
  * GUI: On the backup/restore screen, you can click on the "duplicates" badge next to a game
    to filter the list down to just the games that conflict with it.
    You can click the badge again to reset the filter.
    If the badge is faded out, that means the conflicting saves have been resolved.
  * GUI: On the backup/restore screen, you can use the filter icon to show games
    based on whether they are enabled and whether they have duplicate or ignored saves.
    These filters are reset when you close the program.
  * GUI: On the other screen, there are new options to hide certain kinds of games.
    You can now hide games that are deselected, unchanged, and unscanned.
    These settings are saved between sessions.
  * CLI: Backup comments are now included in the output of the `backups` command.
  * CLI: Registry values now have the `duplicatedBy` field, like files and registry keys.
* Changed:
  * The standalone Linux release is now compiled on Ubuntu 20.04 instead of Ubuntu 18.04
    because of [a change by GitHub](https://github.com/actions/runner-images/issues/6002).
  * When making a new backup for a game,
    if the backup retention limits are reached for that game,
    but your full backup limit is only 1 and you have differential backups enabled,
    Ludusavi will now prune only the oldest differential backup and then make a new differential.

    Previously, Ludusavi would prune the full backup along with its associated differentials
    and then make a new full backup.
    That is still the case when your full backup limit is 2 or more,
    but there is now a special exception when it is only set to 1.
  * GUI: In the save file hierarchy, if a folder is disabled, it will now be collapsed by default.
    Also, when you re-scan a single game, its folders remain expanded or collapsed as you had them
    instead of reverting to the default state.
  * GUI: On the backup and restore screens, the search icon has been replaced with a filter icon,
    which reveals the existing title search along with the new filters described above.
    The sort settings are now always visible, and the "reversed" checkbox is replaced with an ascending/descending icon.
  * GUI: On the backup screen, the gear icon is now on the top row.
* Fixed:
  * Ludusavi only pruned old backups that exceeded your retention settings
    when making a new full backup, but not when making a new differential backup.
    Now, pruning is also performed as needed after a differential backup.
  * The `backups` command needlessly performed a full restoration preview when determining the available backups.
    Now, it only reads the `mapping.yaml` file for each game.
  * When using Heroic on Linux to run Windows games,
    save paths in the game install folders are now checked case-insensitively.
  * When a registry key was toggled off, but one of its values was toggled on,
    the key and value would not be backed up.
    Now, the key will be included along with just the selected values.
    The inverse (key toggled on and values toggled off) was working correctly.
  * GUI: The window would lock up briefly at the start of a backup/restore.
    This was more noticeable on slower systems.
  * GUI: On the backup screen, in the list of saves for each game,
    you can now toggle the file system root when it is on a line of its own.
    Previously, it did not have a checkbox in this case.
  * GUI: On the other screen, backup exclusions could be formatted incorrectly
    if you tried to undo/redo before making any changes to them.
  * GUI: On Mac, if a backup included multiple direct children of the root directory,
    then the first entry in the list would be displayed blank.
    It now correctly shows "/" to indicate the root directory.
  * GUI: On Mac, undo now uses the standard shortcut cmd+z instead of ctrl+z.

## v0.16.0 (2023-03-18)

* Added:
  * Registry values are now listed individually, not just keys.
    This also means you can exclude specific values from the backup.
  * Registry backups now include binary values.
  * Registry backups now handle alternatives to `HKEY_LOCAL_MACHINE\SOFTWARE`.
    For example, when Ludusavi tries to find `HKEY_LOCAL_MACHINE\SOFTWARE\example`,
    it will now also look for:
    * `HKEY_LOCAL_MACHINE\SOFTWARE\Wow6432Node\example`
    * `HKEY_CURRENT_USER\Software\Classes\VirtualStore\MACHINE\SOFTWARE\example`
    * `HKEY_CURRENT_USER\Software\Classes\VirtualStore\MACHINE\SOFTWARE\Wow6432Node\example`
  * GUI: In restore mode, you can create a comment on each backup.
    You can use this to keep track of how each backup reflects your game progress.
  * GUI: You can now reorder custom games, roots, redirects, and ignored paths/registry.
  * CLI: `manifest show` command.
  * CLI: `--compression-level` option for the `backup` command.
  * Updated translations, including new/partial translations for Dutch, French, Russian, and Ukrainian.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

    A translation for Japanese has also been added, but it only has experimental support
    because of a [technical limitation](https://github.com/mtkennerly/ludusavi/issues/9).
    You can enable it by editing the config file directly with language code `ja-JP`,
* Fixed:
  * When a save file failed to be backed up,
    Ludusavi would still record that file in the backup's `mapping.yaml`.
    Because of this, when Ludusavi would later check whether a new backup was needed,
    it would assume that the failed file had been backed up previously.
    Now, failed files are not recorded in `mapping.yaml`,
    so subsequent scans will detect that they still need to be backed up.
  * Heroic Wine/Proton prefixes containing a `pfx` subfolder were not detected.
  * Along with any explicit `installDir` entries in the manifest,
    Ludusavi tries to find each game's install directory based on that game's title.
    However, if that title was not valid as a folder name, then Ludusavi would never find it.
    Now, Ludusavi will ignore characters like `:` and `?` that cannot appear in folder names.
  * For native Linux games installed with the Heroic launcher,
    the `<storeUserId>` path placeholder is now handled in order to detect more saves.
    ([Contributed by sluedecke](https://github.com/mtkennerly/ludusavi/issues/177))
  * Ludusavi currently cannot back up registry keys whose names contain a forward slash.
    This limitation still exists, but now such keys are no longer listed incorrectly as two separate keys.
    This was only a display issue, because such keys were not included in the backup regardless.
  * GUI: If some saves failed to back up, then the scan buttons would stay deactivated,
    and you would have to reopen the program in order to do another scan.
  * GUI: If a game had a new registry value inside of a key that also contained other keys,
    then the game would be flagged as changed, but not the key with the new value.
    Now that values are listed individually, you can tell what changed.
  * GUI: Scrollbar position on the other screen overlapped some content.
  * GUI: Scroll position is once again preserved when switching between screens.
  * GUI: Some inconsistent element sizes and spacing.
* Changed:
  * GUI: Moved roots to the other screen.
  * GUI: Thanks to updates in [Iced](https://github.com/iced-rs/iced):
    * Text fields now have a blinking cursor.
    * Text fields now support shift+click to select text.
  * Thanks to updates in [steamlocate](https://github.com/WilliamVenner/steamlocate-rs),
    the titles of Steam shortcuts for non-Steam games are now looked up case-insensitively.

## v0.15.2 (2022-12-22)

* Fixed:
  * Native registry saves on Windows were not restored.
  * When switching between dropdowns, they would briefly flicker with incorrect content.
  * Game titles starting with a lowercase letter were listed after all titles starting with an uppercase letter.
  * When using the folder picker for roots and custom game files, glob special characters were not escaped.

## v0.15.1 (2022-11-25)

* Fixed:
  * The placeholder `<winProgramData>` was incorrectly interpreted as
    `C:/Windows/ProgramData` when it should have been `C:/ProgramData`.
    This affected the lookup of the normal location on Windows,
    but it did not affect Wine/Proton or VirtualStore paths.
  * For Wine prefixes from Heroic and Wine prefixes passed by CLI,
    the prefix's `*.reg` files were backed up even if the game in question
    was not known to have registry-based saves.
* Changed:
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.15.0 (2022-11-07)

* Added:
  * Steam shortcuts for non-Steam games are now detected.
    On all platforms, the shortcut's "start in" folder is used as the `<base>` path.
    On Linux, the shortcut's app ID is used to check `steamapps/compatdata` for Proton saves.
  * In Heroic roots, Ludusavi can now recognize games by their GOG ID.
    This helps resolve cases where Heroic and Ludusavi use different titles for the same game.
    The CLI `find` command now also has a `--gog-id` option.
  * GUI: On the Steam Deck, an "exit" button has been added to the other screen,
    to make it easier to exit the program while using game mode.
    Ludusavi checks if `/home/deck` exists in order to determine whether it is running on the Steam Deck.
  * CLI: `--config` option to set a custom config directory.
    ([Contributed by sluedecke](https://github.com/mtkennerly/ludusavi/pull/153))
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Changed:
  * Manifest updates now use gzip compression, cutting the download size to about 10% (e.g., 11.4 MiB -> 1.5 MiB).
* Fixed:
  * GUI: Notifications did not disappear when the window was inactive.

## v0.14.0 (2022-10-29)

* Added:
  * Ludusavi now shows which games and files are new/changed compared to the last backup or restore.
    This is indicated by a `+` or `Δ` badge next to applicable games and files.
  * The Heroic launcher is now supported as a root type.
    Both GOG and Epic games are detected, as well as any Wine prefixes (on Linux).
    ([Contributed by sluedecke](https://github.com/mtkennerly/ludusavi/pull/141))
  * Compression levels can now be customized for zip backups.
  * In addition to restoration redirects, there are now also backup redirects and bidirectional redirects.
    The redirect editor is now on the "other" screen instead of the "restore" screen.
  * GUI: On startup, Ludusavi will ask if you'd like to add any missing roots.
    It will remember your choice and won't ask twice for the same root.
  * GUI: The custom games screen now has a button to preview a specific game on demand.
    This lets you preview a custom game even if it's not yet in the backup screen's main list.
  * GUI: When previewing a specific game on demand,
    if it disappears from the list because save data can no longer be found for it,
    then a notification is shown to explain what happened.
  * GUI: The "other" screen now shows when the manifest was last checked/updated.
    There is also a button to refresh on demand.
    While the manifest is updating, a small notification is displayed at the bottom of the window.
  * GUI: Tooltips for some icons that may not be self-explanatory.
  * CLI: `find` command to look up game titles from the manifest.
    This incorporates the `--by-steam-id` option from the `backup`/etc commands
    and adds some new ones, like `--normalized` to look up games by an inexact name.
  * CLI: Backup options: `--format`, `--compression`, `--full-limit`, `--differential-limit`.
  * On startup, Ludusavi will prune any useless blank configurations
    (e.g., roots with a blank path).
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Changed:
  * Increased scanning speed by 10% by avoiding some duplicate path lookups.
  * CLI: Deprecated `--by-steam-id` in the `backup`/`backups`/`restore` commands,
    in favor of the new `find` command.
  * Ludusavi will no longer migrate pre-v0.10.0 configurations to the current location.
  * A new `cache.yaml` is now used for some fields from `config.yaml`,
    specifically the recent game caching and manifest update tracking.
  * On startup, Ludusavi will only check for manifest updates if the last check was 24 hours ago or longer.
    Previously, it would check automatically on every startup.
    This was changed to avoid excess network traffic,
    because the manifest itself will be updated more frequently.
  * GUI: Styling is now more consistent for disabled buttons.
  * GUI: When adding a new root or custom game, the list automatically scrolls to the end.
* Fixed:
  * Backup files did not store the correct modification time on Linux and defaulted to the current time.
    This also affected Windows, but only for zip backups.
    ([Contributed by sluedecke](https://github.com/mtkennerly/ludusavi/pull/136))
  * Zipped backup files did not store the correct permissions on Linux/Mac.
  * Proton and Wine files are now searched case-insensitively on Linux.
  * When Ludusavi tried to find a rough match for an install folder like "Some Game",
    it did not recognize that "Some - Game" was close enough.
  * GUI: Crash if you started a scan, clicked "find roots", and then clicked "cancel"
    while the scan was still ongoing.
  * GUI: When the manifest finished updating in the background,
    any currently open modal would be closed.
  * CLI: If you had the merge option disabled and passed `--merge` to override and enable it,
    it would be respected for the main `--path` folder, but not for game subfolders.
    When Ludusavi detected that a specific game needed a new backup,
    that game's subfolder would be cleared out first.
    If you had the merge option enabled by default, then this did not affect you.
  * CLI mode asked for confirmation when restoring, but backups behaved differently:

    * If the target folder did not exist, then the backup would happen without confirmation.
    * If it did exist, then the `--force` or `--merge` option had to be specified,
      even if you already had merging enabled in your config.

    Now, backups ask for confirmation unless you specify `--force` or `--preview`,
    and the confirmation phrasing is aligned with GUI mode.

## v0.13.1 (2022-09-29)

* Fixed:
  * In GUI mode on windows, an extra console window appeared.
  * In CLI mode, when restoring with redirected paths, the original paths were not shown correctly.

## v0.13.0 (2022-09-28)

* Added:
  * File-based logging in the same directory with the config and manifest files.
    By default, only warnings and errors are logged,
    but you can customize this by setting the `RUST_LOG` environment variable
    (e.g., `RUST_LOG=ludusavi=debug`).
    The most recent 5 log files are kept, rotating on app launch or when a log reaches 10 MiB.
  * On Windows, `%LocalAppData%/VirtualStore` will be checked for potential matches of:
    * `C:/Program Files`
    * `C:/Program Files (x86)`
    * `C:/ProgramData`
    * `C:/Windows`
  * Updated translations.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))
* Changed:
  * Removed the option to "exclude save locations that have only been confirmed on another operating system"
    (config key: `excludeOtherOsData`). This was primarily meant as an optimization for Windows users,
    but in practice, it made little difference on Windows and would rarely be desired on other platforms,
    leading to confusion.
* Fixed:
  * When looking for game install folders during a full scan,
    Ludusavi did not recognize partial folder name matches.
    ([Investigated by sluedecke][https://github.com/mtkennerly/ludusavi/issues/123])
  * When restoring a backup that was made on a different OS,
    it could get stuck before completing the process.
    ([Contributed by Hizoul](https://github.com/mtkennerly/ludusavi/pull/127))
  * GUI: Window now appears immediately and updates the manifest in the background,
    rather than waiting to show the window until the update is complete.
    If there is no local manifest at all, a loading screen is shown while downloading.

## v0.12.1 (2022-08-31)

* Fixed:
  * Updated translations, including new ones for Esperanto and (experimentally) Korean.
    This was meant to be included in 0.12.0, but was missed during the release preparation.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

## v0.12.0 (2022-08-31)

* Added:
  * GUI: Dark theme.
  * GUI: On app startup, games found during the most recent scan will be displayed
    in an "unknown" state, allowing you to re-scan any of those specific games
    on demand without doing a full scan of all games. This is especially useful
    if full scans are slower on your system and you just want to back up one game.
  * GUI: Added an option to re-preview a specific game on demand. This and other
    per-game actions have been grouped into a popup menu to save space. You can
    use keyboard shortcuts to convert the menu into some specific shortcuts:
    * preview: shift
    * backup/restore: ctrl (Mac: cmd)
    * backup/restore without confirmation: ctrl + alt (Mac: cmd + option)
  * CLI: `backups` command to list backups for each game,
    and a `--backup` flag to restore a specific backup by ID.
  * On Linux, GOG roots now additionally check for a `game` subfolder when
    parsing the `<base>` and `<game>` placeholders.
    ([contributed by sluedecke](https://github.com/mtkennerly/ludusavi/pull/121))
  * The Steam root for Flatpak is now auto-detected.
  * If you set the `LUDUSAVI_DEBUG` environment variable, then Ludusavi will
    not detach from the console on Windows. This may be helpful if you want to
    troubleshoot an issue that involves Ludusavi crashing.
* Changed:
  * GUI: You can now use folder pickers while a backup/restore is underway.
  * GUI: If all of a game's files are deselected, then the game will show in the
    same style as a disabled game.
* Fixed:
  * Performance regressions from v0.11.0 related to duplicate detection and root globbing.
  * CLI: `restore --by-steam-id 123` would restore all games with a Steam ID
    instead of just the game with the matching ID 123.
  * CLI: When using `--api`, some non-JSON errors would be printed instead of or
    in addition to the JSON info in certain situations. Now, as long as the CLI
    input itself can be parsed, the output will either be valid JSON or blank.
  * GUI: Cancelling a backup/restore no longer has a long delay.
  * GUI: Doing a full preview, backing up one game, and then doing a full backup
    would trigger a full scan instead of reusing the list from the initial preview.
  * GUI: When backing up a single game by using the button next to its name,
    other games with duplicate files did not update their duplicate status.
  * GUI: When backing up a single game by using the button next to its name,
    if its information changed, then it would immediately re-sort in the list.
    While that made sense, it could be hard to use since you would then have to
    go looking for its new position. Now, the position stays stable unless you
    do a new full scan or manually change the sorting options.

## v0.11.0 (2022-08-20)

* Added:
  * Support for multiple full and differential backups per game.
  * Support for compressed zip backups.
  * Translations for German, Spanish, Filipino, Italian, Polish, and Brazilian Portuguese.
    (Thanks to contributors on the [Crowdin project](https://crowdin.com/project/ludusavi))

    Note that some of these translations are still incomplete. Also, when using some
    translations, GUI sizing may not be optimal, but this will be further refined in the future.

    The following translations have also been added, but only have experimental support
    because of a [technical limitation](https://github.com/mtkennerly/ludusavi/issues/9).
    You can enable them by editing the config file directly: Arabic (`ar-SA`),
    Simplified Chinese (`zh-Hans`)
  * During first-time setup, Ludusavi will now automatically detect roots for
    secondary Steam library folders (Windows/Linux/Mac) and non-default Epic
    install folders (Windows).
  * It is now possible to ignore paths for backup across all games
    and/or for specific games.
  * GUI: Button to back up or restore an individual game from the list on demand.
  * GUI: Button to find and add any missing roots. This is the same functionality
    as the automatic first-time setup, but is now available on demand.
  * Option to sort the game list by file size and to reverse the sorting.
  * Support for Prime Gaming roots.
  * Support for globs in root paths.
  * On Windows, the version field is now set in the executable properties.
  * On Windows, the executable icon is now included in the crates.io release as well.
  * CLI: Blank line between games for better readability.
* Fixed:
  * GUI: Unable to start on KDE 5.25.3 when using Wayland.
  * GUI: Performance has been improved generally, including for very large results (1,000+ games).
  * GUI: Unresponsive while deleting the backup directory with `merge` disabled
    if the folder was very large.
  * GUI: Improved spacing/padding consistency between some elements.
  * Removed `/games` from the end of the default Uplay (Ubisoft Connect) root
    paths. The new default is `C:/Program Files/Ubisoft/Ubisoft Game Launcher`.
  * Crash when launching Ludusavi after the user manually deleted the manifest.
  * If duplicate files were found while a game's file list were already open,
    then the files would not immediately be marked as duplicates until you
    closed and reopened the file list.
* Changed:
  * Localization now uses [Project Fluent](https://projectfluent.org) instead of pure Rust code internally.
    If you'd like to help translate Ludusavi, [check out the Crowdin project](https://crowdin.com/project/ludusavi).
  * Previously, as an optimization, Ludusavi would remember the games it found
    from its first backup/preview and only re-check those games on subsequent
    backups, until certain configuration changes were made (e.g., adding a root).
    However, this had the side effect that newly installed games may not be
    detected right away without an obvious reason why.

    Now, every preview will trigger a full scan. After a preview, doing a backup
    will only include the games found in the preview. If you then do another
    preview or consecutive backup, then it will be a new full scan. This ensures
    Ludusavi will find newly installed games, but it still optimizes for the
    common case of doing a preview immediately followed by a backup.
  * Previously, when Ludusavi backed up a symlink, the backup would contain a
    normal folder with the symlink's name and copies of any files inside of the
    symlink target. If the symlink target itself were also included in the list
    of things to back up, then the same files would be duplicated (once under
    the original directory name and once under the symlink name).

    Now, Ludusavi will still follow symlinks and back up their targets,
    but it will not back up the symlink itself or duplicate the files.
  * When looking for game install folders, Ludusavi previously checked _either_
    the `installDir` entries from the manifest _or_ the game's name, but never
    both at the same time, leading to some missed saves when the `installDir`
    list was incomplete or was not accurate for all stores.

    Now, Ludusavi applies a heuristic to find any install folder that is
    sufficiently similar to the game's title or any known `installDir` value.
    It picks the best match across all games that the install directory
    could possibly represent.
  * Previously, for Steam roots, Ludusavi assumed that the `<storeUserId>`
    would be a series of numbers. However, for some games, this ID may be from
    another launcher and may not conform to those rules. Now, Ludusavi
    just checks for any text, like it does for non-Steam roots.
  * The `merge` setting is now enabled by default.
  * The app window's minimum size has increased from 640x480 to 800x600.
    It may be returned to 640x480 in the future, but there are currently
    some limitations that make it look poor at that size.
  * GUI: Previously, the overall game/size numbers would stay at the values
    from the last scan, and if you started (de)selecting games, then a separate
    badge would show the game/size totals for the new selection. Now, the main
    numbers update progressively and the badge has been removed.
  * GUI: When you launch the program, if the config file is invalid, it gets
    reset with a default copy. Previously, this would delete the invalid copy
    without any means of getting it back. Now, the invalid copy is renamed to
    `config.invalid.yaml` in case you would like to inspect/fix it.

## v0.10.0 (2021-03-12)

* Added:
  * CLI: `--wine-prefix` option for backups.
  * GUI: Root types are now selected via a dropdown instead of radio buttons.
  * Several new root types have been added for various stores, which will allow
    for better store-specific path detection in the future. There is also a
    special type for custom home folders.
  * Custom games can now be individually disabled.
* Changed:
  * Ludusavi now stores its configuration in a more standard location on each
    operating system, instead of always using `~/.config/ludusavi`.
    ([contributed by micke1m](https://github.com/mtkennerly/ludusavi/pull/63))

    * Windows: `%APPDATA%/ludusavi`
    * Linux: `$XDG_CONFIG_HOME/ludusavi` or `~/.config/ludusavi`
    * Mac: `~/Library/Application Support/ludusavi`

    If you've used an older version, your existing configuration will be moved
    automatically to the new location.
  * GUI: Switched to OpenGL by default and upgraded to [Iced 0.2.0](https://crates.io/crates/iced).
  * GUI: Custom games are now more visually distinct from each other.
* Fixed:
  * The 32-bit Windows executable was not properly compatible with 32-bit
    systems.
  * For Proton and Wine, Ludusavi now looks for multiple variations of a few folders:
    * `<winDocuments>` checks `~/Documents` (in addition to `~/My Documents`).
    * `<winAppData>` checks `~/AppData/Roaming` (in addition to `~/Application Data`).
    * `<winLocalAppData>` checks `~/AppData/Local` and `~/Local Settings/Application Data`
      (instead of `~/Application Data`).

## v0.9.0 (2020-08-30)

* Added:
  * An indication when a single file or registry key will be backed up by
    more than one game.
  * CLI: A `complete` command for generating shell completion scripts.
  * GUI: Info about how many games are selected, if the number is different
    than how many games have been processed in the backup/restore.
  * GUI: An edit button next to each game in backup mode to quickly create
    a custom entry for that game, pre-filled with the default info.
  * GUI: A search option to just see games with matching names.
  * GUI: A button to open a game's PCGamingWiki article so that you can more
    easily review or update its information.
* Changed:
  * GUI: Each game's list of files is now a tree with collapsible folders,
    rather than a plain list of full paths. Performance has also been improved
    for very large lists.
  * GUI: The backup/restore confirmation screen now has some extra text that
    recommends doing a preview first.
* Fixed:
  * When using a custom entry to override a known game, some of the original's
    non-overridable data would not be inherited by the custom entry (namely its
    Steam ID and installation directory name).
  * GUI: If the initial full scan found saves for a game, and then you created
    a custom entry for that game such that no saves would be found, did a scan,
    deleted the custom entry, and did another scan, then that scan would not
    find any files for the game. Now, it will correctly revert to the standard
    data for that game immediately.

## v0.8.0 (2020-08-10)

* Added:
  * If you create a file called `ludusavi.portable` in the same location as
    the executable, then Ludusavi will store its config file and the manifest
    there as well.
* Fixed:
  * Read-only files could only be backed up once, since the original backup
    could not be replaced by a newer copy, and you could not restore a backup
    if the original file was read-only. Now, Ludusavi will try to unset the
    read-only flag on backups before replacing them with newer backups, and
    it will try to unset the flag on target files before restoring a backup.
  * Invalid paths like `C:\Users\Foo\Documents\C:\Users\Foo` would be shortened
    to just `C:\Users\Foo`, which could cause irrelevant files to be backed up.
    Now, the extraneous `C:` will be converted to `C_` so that it simply won't
    match any files or directories.
  * When some games were deselected, the disk space display only showed units
    for the total space, not the used space, which could lead to it showing
    "1.42 of 1.56 GiB", where 1.42 was actually MiB and not GiB.
    Units are now shown for both sides.
* Changed:
  * When backing up or restoring a file, if it already exists with the correct
    content, then Ludusavi won't re-copy it.
  * In GUI mode, Ludusavi now tries to be smarter about when a full scan is
    needed. Previously, every backup and backup preview would trigger a full
    scan. Now, Ludusavi will remember which games it found and only re-scan
    those games (until you change your roots, change the "other" settings,
    or reopen the program).
  * In CLI mode, `--try-update` will use a default, empty manifest if there is
    no local copy of the manifest and it cannot be downloaded.

## v0.7.0 (2020-08-01)

**The backup structure has changed! Read below for more detail.**

* Added:
  * Backup option to exclude save locations that are only confirmed for
    another operating system.
  * Backup option to exclude store screenshots.
  * `--try-update` flag for backups via CLI.
* Fixed:
  * When starting the GUI, if Ludusavi could not check for a manifest update
    (e.g., because your Internet is down), then it would default to an empty
    manifest even if you already had a local copy that was downloaded before.
    Now, it will use the local copy even if it can't check for updates.
* Changed:
  * Backup structure is now human-readable.
  * App window now has a minimum size, 640x480.
    (Note: For now, the crates.io release will not have a minimum size.)
  * File size units are now adjusted based on the size, rather than always using MiB.
    ([contributed by wtjones](https://github.com/mtkennerly/ludusavi/pull/32))

### New backup structure
Previously, Ludusavi used Base64 to encode game names and original paths when
organizing backups. There were some technical advantages of that approach, but
it was not easy to understand, and there was a technical flaw because Base64
output can include `/`, which isn't safe for folder or file names.

Therefore, Ludusavi now organizes backups like this, in a way that is easier
to read and understand:

```
C:/somewhere/the-backup-folder/
  Game 1 Name/
    mapping.yaml
    registry.yaml
    drive-C/  # drive-0 on Linux and Mac
      Users/
        ...
      Program Files/
        Steam/
          ...
```

The name of each game's folder is as close to the real title as possible,
except for replacing some special characters with `_`. Ultimately, Ludusavi
doesn't care much about the folder name and mainly looks for `mapping.yaml`,
which contains some metadata that Ludusavi needs. If a game has any Windows
registry data to back up, then there will also be a `registry.yaml` file.
Within each drive folder, everything is simply organized exactly like it
already is on your computer.

If you need to restore a previous backup, then please use Ludusavi v0.6.0
to do the restoration first, then migrate to Ludusavi v0.7.0 and create a
new backup.

You can [read more here](https://github.com/mtkennerly/ludusavi/issues/29)
about the background of this change. Be assured that this sort of disruptive
change is not taken lightly, but may happen in some cases until Ludusavi
reaches version 1.0.0.

## v0.6.0 (2020-07-29)

* Added:
  * Option to merge into an existing backup directory.
  * `--api` flag in CLI mode.
  * `--by-steam-id` flag in CLI mode.
* Fixed:
  * Registry values of type `EXPAND_SZ` and `MULTI_SZ` were converted to `SZ` when restored.
* Changed:
  * On Windows, the program icon is now embedded in the executable so that
    you can see it in the file browser as well.

## v0.5.0 (2020-07-25)

* Added:
  * Support for custom games.
  * Icons for several buttons.
  * An icon for Ludusavi itself.
    (Note: For now, the crates.io release will not show this icon.)
  * Support for cutting in text fields.
    (Note: For now, the crates.io release will copy text instead of cutting it.)
  * More buttons for browsing folders.
  * Support for `~` (user home directory) in redirects.
  * Support for `.` and `..` path segments when the path does not exist.
* Fixed:
  * On Windows, long paths can now be backed up without issue.
  * When backing up files, the Base64-encoded name now preserves the original
    file's actual capitalization, rather than the expected capitalization
    from the manifest.
  * There was a rare issue related to the above point where some files could be
    backed up twice, once with the original capitalization and once with the
    expected capitalization.
  * The CLI required the backup `--path` to already exist.
  * Keyboard shortcuts didn't work in redirect fields.
  * Registry keys were not backed up if the parent key had no values.
  * CLI mode would panic when restoring if a non-Base64-encoded file were
    present in the source folder. Now, such files will be reported as an error.
* Changed:
  * The configuration auto-save is now more predictable. All config changes
    are now saved immediately.
  * When a game has registry data to back up, registry.yaml no longer includes
    unnecessary fields and is now sorted alphabetically. This means that identical
    registry content will produce an identical registry.yaml across backups.
  * The progress bar is now shown on all screens.

## v0.4.0 (2020-07-21)

* Added the ability to select and deselect specific games.
* Added the ability to restore to different folders via redirects.
* Added indicators for how much disk space is used by the files.
* Added indicators in the GUI when files fail to process.
* Added a browse button for folders.
* Replaced the "=> Restore" and "=> Backup" buttons with a navigation bar.
* Redesigned the confirmation and error screens so that the buttons are shown
  below the text, which helps to prevent any accidental clicks before reading.
* Narrowed how Steam IDs are substituted in paths to avoid false positives.
* Fixed an issue where restore mode in the GUI would get stuck showing an
  "in progress" state if the source path had no subdirectories.

## v0.3.0 (2020-07-12)

* Added command line interface.
* Added common roots for GOG Galaxy on Windows.
* Added copy/undo/redo shortcuts in text fields. Cutting is not yet supported
  because of some limitations in the GUI library.
* Changed scrollbar style so that it's more obvious what's scrollable.
* Changed build process to avoid potential "VCRUNTIME140_1.dll was not found"
  error on Windows.

## v0.2.0 (2020-07-06)

* Added core backup/restore functionality.
* Added support for saves in the Windows registry.
* Added support for Steam + Proton saves.
* Added support for Steam screenshots.

## v0.1.0 (2020-06-20)

* Initial release.
* Just a prototype/mock-up and not yet functional.

ludusavi = Ludusavi

language = Language
language-font-compatibility = Some languages may require a custom font.
font = Font

cli-backup-target-already-exists = The backup target already exists ( {$path} ). Either choose a different --path or delete it with --force.
cli-unrecognized-games = No info for these games:
cli-confirm-restoration = Do you want to restore from {$path}?
cli-unable-to-request-confirmation = Unable to request confirmation.
    .winpty-workaround = If you are using a Bash emulator (like Git Bash), try running winpty.

badge-failed = FAILED
badge-duplicates = DUPLICATES
badge-duplicated = DUPLICATED
badge-ignored = IGNORED
badge-redirected-from = FROM: {$path}

some-entries-failed = Some entries failed to process; look for {badge-failed} in the output for details. Double check whether you can access those files or whether their paths are very long.

cli-game-line-item-redirected = Redirected from: {$path}
cli-summary =
    .succeeded =
        Overall:
          Games: {$processed-games}
          Size: {$processed-size}
          Location: {$path}
    .failed =
        Overall:
          Games: {$processed-games} of {$total-games}
          Size: {$processed-size} of {$total-size}
          Location: {$path}

button-backup = Back up
button-preview = Preview
button-restore = Restore
button-nav-backup = BACKUP MODE
button-nav-restore = RESTORE MODE
button-nav-custom-games = CUSTOM GAMES
button-nav-other = OTHER
button-add-root = Add root
button-find-roots = Find roots
button-add-redirect = Add redirect
button-add-game = Add game
button-continue = Continue
button-cancel = Cancel
button-cancelling = Cancelling...
button-okay = Okay
button-select-all = Select all
button-deselect-all = Deselect all
button-enable-all = Enable all
button-disable-all = Disable all

no-roots-are-configured = Add some roots to back up even more data.

config-is-invalid = Error: The config file is invalid.
manifest-is-invalid = Error: The manifest file is invalid.
manifest-cannot-be-updated = Error: Unable to check for an update to the manifest file. Is your Internet connection down?
cannot-prepare-backup-target = Error: Unable to prepare backup target (either creating or emptying the folder). If you have the folder open in your file browser, try closing it: {$path}
restoration-source-is-invalid = Error: The restoration source is invalid (either doesn't exist or isn't a directory). Please double check the location: {$path}
registry-issue = Error: Some registry entries were skipped.
unable-to-browse-file-system = Error: Unable to browse on your system.
unable-to-open-directory = Error: Unable to open directory:
unable-to-open-url = Error: Unable to open URL:

processed-games = {$total-games} {$total-games ->
    [one] game
    *[other] games
}
processed-games-subset = {$processed-games} of {$total-games} {$total-games ->
    [one] game
    *[other] games
}
processed-size-subset = {$processed-size} of {$total-size}

field-backup-target = Back up to:
toggle-backup-merge = Merge
field-restore-source = Restore from:
field-custom-files = Paths:
field-custom-registry = Registry:
field-search = Search:
field-sort = Sort:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-custom-game-name =
    .placeholder = Name
field-search-game-name =
    .placeholder = Name
field-backup-excluded-items = Backup exclusions:

store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home folder
store-other-wine = Wine prefix
store-other = Other

sort-name = Name
sort-size = Size
sort-reversed = Reversed

explanation-for-exclude-other-os-data =
    In backups, exclude save locations that have only been confirmed on another
    operating system. Some games always put saves in the same place, but the
    locations may have only been confirmed for a different OS, so it can help
    to check them anyway. Excluding that data may help to avoid false positives,
    but may also mean missing out on some saves. On Linux, Proton saves will
    still be backed up regardless of this setting.

explanation-for-exclude-store-screenshots =
    In backups, exclude store-specific screenshots. Right now, this only applies
    to {store-steam} screenshots that you've taken. If a game has its own built-in
    screenshot functionality, this setting will not affect whether those
    screenshots are backed up.

consider-doing-a-preview =
    If you haven't already, consider doing a preview first so that there
    are no surprises.

confirm-backup =
    Are you sure you want to proceed with the backup? {$path-action ->
        [merge] New save data will be merged into the target folder
        [recreate] The target folder will be deleted and recreated from scratch
        *[create] The target folder will be created
    }:

    {$path}

    {consider-doing-a-preview}

confirm-restore =
    Are you sure you want to proceed with the restoration?
    This will overwrite any current files with the backups from here:

    {$path}

    {consider-doing-a-preview}

confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.

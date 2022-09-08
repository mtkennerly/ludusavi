ludusavi = Ludusavi

language = Language
font = Font
game-name = Name
total-games = Games
file-size = Size
file-location = Location
overall = Overall

cli-backup-target-already-exists = The backup target already exists ( {$path} ). Either choose a different --path or delete it with --force.
cli-unrecognized-games = No info for these games:
cli-confirm-restoration = Do you want to restore from {$path}?
cli-unable-to-request-confirmation = Unable to request confirmation.
    .winpty-workaround = If you are using a Bash emulator (like Git Bash), try running winpty.
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.

badge-failed = FAILED
badge-duplicates = DUPLICATES
badge-duplicated = DUPLICATED
badge-ignored = IGNORED
badge-redirected-from = FROM: {$path}

some-entries-failed = Some entries failed to process; look for {badge-failed} in the output for details. Double check whether you can access those files or whether their paths are very long.

cli-game-line-item-redirected = Redirected from: {$path}

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
button-customize = Customize

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
field-backup-excluded-items = Backup exclusions:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:

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

sort-reversed = Reversed

backup-format-simple = Simple
backup-format-zip = Zip

compression-none = None
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd

theme = Theme
theme-light = Light
theme-dark = Dark

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
        [merge] New save data will be merged into the target folder:
        [recreate] The target folder will be deleted and recreated from scratch:
        *[create] The target folder will be created:
    }

confirm-restore =
    Are you sure you want to proceed with the restoration?
    This will overwrite any current files with the backups from here:

confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
preparing-backup-target = Preparing backup directory...
updating-manifest = Updating manifest...

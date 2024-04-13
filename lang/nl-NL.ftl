ludusavi = Ludusavi
language = Taal
game-name = Naam
total-games = Games
file-size = Grootte
file-location = Locatie
overall = Totaal
status = Status
cli-unrecognized-games = Geen informatie voor deze games:
cli-unable-to-request-confirmation = Er kan niet om bevestiging worden gevraagd.
    .winpty-workaround = Als je een Bash-emulator, zoals Git Bash, gebruikt, probeer dan winpty uit te voeren.
cli-backup-id-with-multiple-games = De back-up-id kan niet worden opgegeven bij het herstellen van meerdere games.
cli-invalid-backup-id = Ongeldige back-up-id.
badge-failed = MISLUKT
badge-duplicates = DUPLICATEN
badge-duplicated = GEDUPLICEERD
badge-ignored = GENEGEERD
badge-redirected-from = VAN: { $path }
badge-redirecting-to = NAAR: { $path }
some-entries-failed = Sommige items konden niet worden verwerkt - zoek naar { badge-failed } in de uitvoer om de details te bekijken. Controleer nogmaals of je toegang hebt tot deze bestanden of of hun paden erg lang zijn.
cli-game-line-item-redirected = Doorverwezen van: { $path }
cli-game-line-item-redirecting = Doorverwijzen naar: { $path }
button-backup = Back-up
button-preview = Voorvertoning
button-restore = Herstellen
button-nav-backup = BACK-UPMODUS
button-nav-restore = HERSTELMODUS
button-nav-custom-games = ANDERE SPELLEN
button-nav-other = OVERIG
button-add-game = Game toevoegen
button-continue = Doorgaan
button-cancel = Annuleren
button-cancelling = Bezig met annuleren…
button-okay = Oké
button-select-all = Alles selecteren
button-deselect-all = Niets selecteren
button-enable-all = Alles inschakelen
button-disable-all = Alles uitschakelen
button-customize = Aanpassen
button-exit = Afsluiten
button-comment = Comment
button-lock = Lock
button-unlock = Unlock
# This opens a download page.
button-get-app = Get { $app }
button-validate = Validate
no-roots-are-configured = Voeg hoofdmappen toe om meer gegevens te back-uppen.
config-is-invalid = Foutmelding: het configuratiebestand is ongeldig.
manifest-is-invalid = Foutmelding: het manifestbestand is ongeldig.
manifest-cannot-be-updated = Foutmelding: er kan niet worden gecontroleerd op een update van het manifestbestand. Ben je verbonden met het internet?
cannot-prepare-backup-target = Error: Unable to prepare backup target (either creating or emptying the folder). If you have the folder open in your file browser, try closing it: { $path }
restoration-source-is-invalid = Error: The restoration source is invalid (either doesn't exist or isn't a directory). Please double check the location: { $path }
registry-issue = Error: Some registry entries were skipped.
unable-to-browse-file-system = Error: Unable to browse on your system.
unable-to-open-directory = Error: Unable to open directory:
unable-to-open-url = Error: Unable to open URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] game
       *[other] games
    }
processed-games-subset =
    { $processed-games } of { $total-games } { $total-games ->
        [one] game
       *[other] games
    }
processed-size-subset = { $processed-size } of { $total-size }
field-backup-target = Back up to:
field-restore-source = Restore from:
field-custom-files = Paths:
field-custom-registry = Registry:
field-sort = Sort:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-roots = Roots:
field-backup-excluded-items = Backup exclusions:
field-redirects = Redirects:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Level:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Checked
# This shows the time when we found an update to the manifest.
label-updated = Updated
label-new = New
label-removed = Removed
label-comment = Comment
label-unchanged = Unchanged
label-scan = Scan
label-filter = Filter
label-unique = Unique
label-complete = Complete
label-partial = Partial
label-enabled = Enabled
label-disabled = Disabled
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remote
label-remote-name = Remote name
label-folder = Folder
# An executable file
label-executable = Executable
# Options given to a command line program
label-arguments = Arguments
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Port
label-username = Username
label-password = Password
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Provider
label-custom = Custom
label-none = None
label-change-count = Changes: { $total }
label-unscanned = Unscanned
# This refers to a local file on the computer
label-file = File
label-game = Game
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Original name
store-ea = EA
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-legendary = Legendary
store-lutris = Lutris
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home folder
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine prefix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows drive
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux drive
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac drive
store-other = Other
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
redirect-bidirectional = Bidirectional
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
prefer-alias-display = Display alias instead of original name
explanation-for-exclude-store-screenshots = In backups, exclude store-specific screenshots
consider-doing-a-preview =
    If you haven't already, consider doing a preview first so that there
    are no surprises.
confirm-backup =
    Are you sure you want to proceed with the backup? { $path-action ->
        [merge] New save data will be merged into the target folder:
       *[create] The target folder will be created:
    }
confirm-restore =
    Are you sure you want to proceed with the restoration?
    This will overwrite any current files with the backups from here:
confirm-cloud-upload =
    Do you want to replace your cloud files with your local files?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to replace your local files with your cloud files?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
loading = Loading...
preparing-backup-target = Preparing backup directory...
updating-manifest = Updating manifest...
no-cloud-changes = No changes to synchronize
backups-are-valid = Your backups are valid.
backups-are-invalid =
    These games' backups appear to be invalid.
    Do you want to create new full backups for these games?
saves-found = Save data found.
no-saves-found = No save data found.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = no confirmation
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.
game-is-unrecognized = Ludusavi does not recognize this game.
game-has-nothing-to-restore = This game does not have a backup to restore.
launch-game-after-error = Launch the game anyway?
game-did-not-launch = Game failed to launch.
back-up-specific-game =
    .confirm = Back up save data for { $game }?
    .failed = Failed to back up save data for { $game }
restore-specific-game =
    .confirm = Restore save data for { $game }?
    .failed = Failed to restore save data for { $game }

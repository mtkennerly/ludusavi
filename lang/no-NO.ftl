ludusavi = Ludusavi
language = Språk
game-name = Navn
total-games = Spill
file-size = Størrelse
file-location = Plassering
overall = Generelt
status = Status
cli-unrecognized-games = Ingen informasjon for disse spillene:
cli-unable-to-request-confirmation = Kunne ikke be om bekreftelse.
    .winpty-workaround = Hvis du bruker en Bash-emulator (som f.eks. Git Bash), kan du prøve å kjøre winpty.
cli-backup-id-with-multiple-games = Kan ikke spesifisere sikkerhetskopi-ID når man gjenoppretter flere spill.
cli-invalid-backup-id = Ugyldig sikkerhetskopi-ID.
badge-failed = FEILET
badge-duplicates = DUPLIKATER
badge-duplicated = DUPLISERTE
badge-ignored = IGNORERTE
badge-redirected-from = FRA: { $path }
badge-redirecting-to = TIL: { $path }
some-entries-failed = Noen oppføringer feilet med å prosessere; se etter { badge-failed } i utdataen for detaljer. Dobbel sjekk om du har tilgang til filene, eller om filstiene for de filene er veldig lange.
cli-game-line-item-redirected = Omdirigert fra: { $path }
cli-game-line-item-redirecting = Omdirigerer til: { $path }
button-backup = Sikkerhetskopier
button-preview = Forhåndsvisning
button-restore = Gjenopprett
button-nav-backup = SIKKERHETSKOPI-MODUS
button-nav-restore = GJENOPPRETTINGS-MODUS
button-nav-custom-games = ANDRE SPILL
button-nav-other = ANNET
button-add-game = Legg til spill
button-continue = Fortsett
button-cancel = Avbryt
button-cancelling = Avbryter...
button-okay = Okei
button-select-all = Velg alt
button-deselect-all = Velg bort alt
button-enable-all = Aktiver alle
button-disable-all = Deaktiver alle
button-customize = Endre
button-exit = Avslutt
button-comment = Kommentar
button-lock = Lås
button-unlock = Lås opp
# This opens a download page.
button-get-app = Få { $app }
button-validate = Valider
button-override-manifest = Overstyr manifest
button-extend-manifest = Utvid manifest
button-sort = Sorter
button-download = Last ned
button-upload = Last opp
button-ignore = Ignorer
no-roots-are-configured = Legg til noen rot-filstier for å sikkerhetskopiere enda mer lagringsdata.
config-is-invalid = Feil: Konfigurasjons-filen er ugyldig.
manifest-is-invalid = Feil: manifest filen er ugyldig.
manifest-cannot-be-updated = Feil: Kunne ikke sjekke om det er oppdateringer i manifest filen. Er internett-tilkoblingen din nede?
cannot-prepare-backup-target = Feil: Kunne ikke klargjøre sikkerhetskopi-målet (enten ved opprettelse eller tømming av mappen). Hvis du har mappen åpen i filutforskeren din, prøv å lukke den: { $path }
restoration-source-is-invalid = Feil: Gjenopprettingskilden er ugyldig (enten finnes den ikke, eller er ikke en filsti.) Vennligst dobbeltsjekk plasseringen: { $path }
registry-issue = Feil: Noen registeroppføringer ble hoppet over.
unable-to-browse-file-system = Feil: Kunne ikke søke i systemet ditt.
unable-to-open-directory = Feil: Kunne ikke åpne filsti:
unable-to-open-url = Feil: Kunne ikke åpne URL:
unable-to-configure-cloud = Kunne ikke konfigurere sky.
unable-to-synchronize-with-cloud = Kunne ikke synkronisere med sky.
cloud-synchronize-conflict = Dine lokale og sky -sikkerhetskopier har konflikter. Gjør en opplastning eller nedlastning for å løse dette problemet.
command-unlaunched = Kommando ble ikke startet: { $command }
command-terminated = Kommando ble plutselig avbrutt: { $command }
command-failed = Kommando feilet med koden: { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] game
       *[other] games
    }
processed-games-subset = { $processed-games } av { $total-games } spill
processed-size-subset = { $processed-size } av { $total-size }
field-backup-target = Sikkerhetskopier til:
field-restore-source = Gjennoprett fra:
field-custom-files = Plasseringer:
field-custom-registry = Register:
field-sort = Sorter:
field-redirect-source =
    .placeholder = Kilde (original plassering)
field-redirect-target =
    .placeholder = Mål (ny plassering)
field-roots = Rot-filstier:
field-backup-excluded-items = Sikkerhetskopi-eksluderinger:
field-redirects = Omdirigeringer:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differensial:
field-backup-format = Format:
field-backup-compression = Komprimering:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Nivå:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Sjekket
# This shows the time when we found an update to the manifest.
label-updated = Oppdatert
label-new = Ny
label-removed = Fjernet
label-comment = Kommentar
label-unchanged = Uforandret
label-backup = Sikkerhetskopi
label-scan = Skann
label-filter = Filter
label-unique = Unik
label-complete = Ferdig
label-partial = Delvis
label-enabled = Aktivert
label-disabled = Deaktivert
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Tråder
label-cloud = Sky
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remote
label-remote-name = Remote name
label-folder = Mappe
# An executable file
label-executable = Executable
# Options given to a command line program
label-arguments = Argumenter
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Vert
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Port
label-username = Brukernavn
label-password = Passord
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Provider
label-custom = Custom
label-none = Ingen
label-change-count = Endringer: { $total }
label-unscanned = Uskannet
# This refers to a local file on the computer
label-file = Fil
label-game = Spill
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Original navn
# Which manifest a game's data came from
label-source = Kilde
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primær manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Intergrering
# This is a folder name where a specific game is installed
label-installed-name = Installert navn
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
store-other-home = Hjemmappe
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine-prefiks
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows stasjon
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux stasjon
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac stasjon
store-other = Annet
backup-format-simple = Enkel
backup-format-zip = Zip
compression-none = Ingen
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflatere
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Lys
theme-dark = Mørk
redirect-bidirectional = Bidirectional
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = Vis uendrede spill
show-unscanned-games = Vis uskannede spill
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
prefer-alias-display = Display alias instead of original name
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = In backups, exclude store-specific screenshots
explanation-for-exclude-cloud-games = Ikke sikkerhetskopier spill med sky-støtte for disse plattformene
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
prefix-error = Feil: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.
game-is-unrecognized = Ludusavi does not recognize this game.
game-has-nothing-to-restore = This game does not have a backup to restore.
launch-game-after-error = Åpne spillet uansett?
game-did-not-launch = Game failed to launch.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Sikkerhetskopier lagringsdata for { $game }?
    .failed = Feilet med å sikkerhetskopiere lagringsdata for { $game }
restore-specific-game =
    .confirm = Gjennoprett lagringsdata for { $game }?
    .failed = Feilet med å gjennoprette lagringsdata for { $game }
new-version-check = Sjekk for programoppdateringer automatisk
new-version-available = En programoppdatering er tilgjenlig: { $version }. Vil du se utgivelses-notatene?
custom-game-will-override = This custom game overrides a manifest entry
custom-game-will-extend = This custom game extends a manifest entry
operation-will-only-include-listed-games = This will only process the games that are currently listed

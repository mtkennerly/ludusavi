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
button-nav-custom-games = TILPASSEDE SPILL
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
processed-games = { $total-games } spill
processed-games-subset = { $processed-games } av { $total-games } spill
processed-size-subset = { $processed-size } av { $total-size }
field-backup-target = Sikkerhetskopier til:
field-restore-source = Gjenopprett fra:
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
label-remote = Ekstern
label-remote-name = Eksternt navn
label-folder = Mappe
# An executable file
label-executable = Kjørbar fil
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
label-provider = Leverandør
label-custom = Tilpasset
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
redirect-bidirectional = Bidireksjonell
reverse-redirects-when-restoring = Omvendt rekkefølge av omdirigeringer ved gjenoppretting
show-disabled-games = Vis deaktiverte spill
show-unchanged-games = Vis uendrede spill
show-unscanned-games = Vis uskannede spill
override-max-threads = Overskriv maks antall tråder
synchronize-automatically = Synkroniser automatisk
prefer-alias-display = Vis alias i stedet for originalt navn
skip-unconstructive-backups = Hopp over sikkerhetskopiering når data skal ha blitt fjernet, men ikke lagt til eller oppdatert
explanation-for-exclude-store-screenshots = I sikkerhetskopier; ekskluder butikk-spesifikke skjermdumper
explanation-for-exclude-cloud-games = Ikke sikkerhetskopier spill med sky-støtte for disse plattformene
consider-doing-a-preview = Hvis du ikke allerede har gjort det, bør du vurdere å gjøre en forhåndsvisning først, slik at det er ingen overraskelser.
confirm-backup =
    Er du sikker på at du vil fortsette med sikkerhetskopieringen? { $path-action ->
        [merge] Nye lagringsdata kommer til å bli slått sammen med målmappen:
       *[create] Målmappen kommer til å bli opprettet:
    }
confirm-restore = Er du sikker på at du vil fortsette med gjenopprettingen? Dette kommer til å overskrive gjeldende åpne filer med sikkerhetskopier fra her:
confirm-cloud-upload =
    Vil du erstatte sky-filene dine med dine lokale filer? Dine sky-filer ({ $cloud-path }) kommer til å bli en eksakt kopi av dine lokale filer ({ $local-path }).
    Filer i skyen kommer til å bli oppdatert eller slettet etter behov.
confirm-cloud-download =
    Vil du erstatte dine lokale filer med sky-filene dine? Dine lokale filer ({ $cloud-path }) kommer til å bli en eksakt kopi av dine sky-filer ({ $local-path }).
    Lokale filer kommer til å bli oppdatert eller slettet etter behov.
confirm-add-missing-roots = Legg til disse rot-filstiene?
no-missing-roots = Ingen ytterlige rot-filstier funnet.
loading = Laster...
preparing-backup-target = Forbereder sikkerhetskopi-filsti...
updating-manifest = Oppdaterer manifest...
no-cloud-changes = Ingen endringer å synkronisere
backups-are-valid = Sikkerhetskopiene dine er ugyldige.
backups-are-invalid = Sikkerhetskopiene til disse spillene ser ut til å være ugyldige. Vil du lage nye fullstendige sikkerhetskopier for disse spillene?
saves-found = Lagringsdata funnet.
no-saves-found = Ingen lagringsdata funnet.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = ingen bekreftelse
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = omstart nødvendig
prefix-error = Feil: { $message }
prefix-warning = Advarsel: { $message }
cloud-app-unavailable = Sky-sikkerhetskopier er deaktivert på grunn av at { $app } ikke er tilgjengelig.
cloud-not-configured = Sky-sikkerhetskopier er deaktivert på grunn av at ingen sky-systemer er konfigurert.
cloud-path-invalid = Sky-sikkerhetskopier er deaktivert på grunn av at sikkerhetskopi-filstien er ugyldig.
game-is-unrecognized = Ludusavi kjenner ikke igjen spillet.
game-has-nothing-to-restore = Dette spillet har ikke en sikkerhetskopi å gjenopprette.
launch-game-after-error = Åpne spillet uansett?
game-did-not-launch = Spill feilet med å starte.
backup-is-newer-than-current-data = Den eksisterende sikkerhetskopien er nyere enn de nåværende dataene.
backup-is-older-than-current-data = Den eksisterende sikkerhetskopien er gamlere enn de nåværende dataene.
back-up-specific-game =
    .confirm = Sikkerhetskopier lagringsdata for { $game }?
    .failed = Feilet med å sikkerhetskopiere lagringsdata for { $game }
restore-specific-game =
    .confirm = Gjenopprett lagringsdata for { $game }?
    .failed = Feilet med å gjenopprette lagringsdata for { $game }
new-version-check = Sjekk for programoppdateringer automatisk
new-version-available = En programoppdatering er tilgjenglig: { $version }. Vil du se utgivelses-notatene?
custom-game-will-override = Dette tilpassede spillet overskriver en manifest oppføring
custom-game-will-extend = Dette tilpassede spillet utvider en manifest oppføring
operation-will-only-include-listed-games = Dette kommer bare til å prosessere spillene som er for øyeblikket oppført

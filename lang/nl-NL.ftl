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
button-comment = Commentaar
button-lock = Vergrendel
button-unlock = Ontgrendel
# This opens a download page.
button-get-app = Verkrijg { $app }
button-validate = Controleer
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sorteren
button-download = Downloaden
button-upload = Uploaden
button-ignore = Negeren
no-roots-are-configured = Voeg hoofdmappen toe om meer gegevens te back-uppen.
config-is-invalid = Foutmelding: het configuratiebestand is ongeldig.
manifest-is-invalid = Foutmelding: het manifestbestand is ongeldig.
manifest-cannot-be-updated = Foutmelding: er kan niet worden gecontroleerd op een update van het manifestbestand. Ben je verbonden met het internet?
cannot-prepare-backup-target = Foutmelding: Het back-updoelwit kon niet worden voorbereid (de map kon niet worden aan- of leeggemaakt). Als je de map open hebt in je bestandsbrowser, probeer deze dan te sluiten: { $path }
restoration-source-is-invalid = Foutmelding: De herstelbron is ongeldig (deze bestaat niet of is niet een map). Controleer de locatie: { $path }
registry-issue = Foutmelding: Sommige register-entries zijn overgeslagen.
unable-to-browse-file-system = Foutmelding: Systeem kon niet worden doorzocht.
unable-to-open-directory = Foutmelding: Map kon niet geopend worden:
unable-to-open-url = Foutmelding: URL kon niet geopend worden:
unable-to-configure-cloud = Cloud kon niet ingesteld worden.
unable-to-synchronize-with-cloud = Cloud kon niet gesynchroniseerd worden.
cloud-synchronize-conflict = Je lokale back-ups en cloudback-ups zijn met elkaar in conflict. Voer een upload of download uit om dit op te lossen.
command-unlaunched = Opdracht is niet gestart: { $command }
command-terminated = Opdracht plotseling beëindigd: { $command }
command-failed = Opdracht mislukt met code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] spel
       *[other] spellen
    }
processed-games-subset =
    { $processed-games } van { $total-games } { $total-games ->
        [one] spel
       *[other] spellen
    }
processed-size-subset = { $processed-size } van { $total-size }
field-backup-target = Back-up naar:
field-restore-source = Herstel vanuit:
field-custom-files = Paden:
field-custom-registry = Register:
field-sort = Sorteer:
field-redirect-source =
    .placeholder = Bron (originele locatie)
field-redirect-target =
    .placeholder = Doelwit (nieuwe locatie)
field-roots = Stammen:
field-backup-excluded-items = Back-upuitsluitingen:
field-redirects = Doorverwijzingen:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Volledig:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differentiaal:
field-backup-format = Formaat:
field-backup-compression = Compressie:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Niveau:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Gecontroleerd
# This shows the time when we found an update to the manifest.
label-updated = Bijgewerkt
label-new = Nieuw
label-removed = Verwijderd
label-comment = Commentaar
label-unchanged = Onveranderd
label-backup = Backup
label-scan = Scan
label-filter = Filter
label-unique = Uniek
label-complete = Compleet
label-partial = Gedeeltelijk
label-enabled = Ingeschakeld
label-disabled = Uitgeschakeld
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Externe server
label-remote-name = Externe-servernaam
label-folder = Map
# An executable file
label-executable = Uitvoerbaar bestand
# Options given to a command line program
label-arguments = Argumenten
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Poort
label-username = Gebruikersnaam
label-password = Wachtwoord
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Aanbieder
label-custom = Aangepast
label-none = Geen
label-change-count = Veranderingen: { $total }
label-unscanned = Ongescand
# This refers to a local file on the computer
label-file = Bestand
label-game = Spel
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Originele naam
# Which manifest a game's data came from
label-source = Bron
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primary manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integratie
# This is a folder name where a specific game is installed
label-installed-name = Geïnstalleerde naam
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
store-other-home = Thuismap
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wineprefix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windowsschijf
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linuxschijf
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Macschijf
store-other = Anders
backup-format-simple = Simpel
backup-format-zip = Zip
compression-none = Geen
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Thema
theme-light = Licht
theme-dark = Donker
redirect-bidirectional = Bidirectioneel
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = Toon onveranderde spellen
show-unscanned-games = Toon ongescande spellen
override-max-threads = Overschrijf maximale threads
synchronize-automatically = Synchroniseer automatisch
prefer-alias-display = Toon alias in plaats van originele naam
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = Sluit winkel-specifieke schermafdrukken uit van back-up
explanation-for-exclude-cloud-games = Maak geen back-ups van spellen met cloud-ondersteuning op deze platforms
consider-doing-a-preview = Als je dat nog niet gedaan hebt, bekijk dan eerst een vooruitzicht zodat er geen verrassingen zijn.
confirm-backup =
    Weet je zeker dat je verder wil gaan met de back-up? { $path-action ->
        [merge] Nieuwe savedata wordt samengevoegd met de doelmap:
       *[create] De doelmap wordt aangemaakt:
    }
confirm-restore =
    Weet je zeker dat je verder wil gaan met het herstel?
    Bestaande bestanden zullen worden overschreven door de volgende back-ups:
confirm-cloud-upload =
    Wil je je cloudbestanden vervangen met je lokale bestanden?
    Je cloudbestanden ({ $cloud-path }) worden dan een exacte kopie van je lokale bestanden ({ $local-path }).
    Cloudbestanden worden bijgewerkt of verwijderd waar nodig.
confirm-cloud-download =
    Wil je je lokale bestanden vervangen met je cloudbestanden?
    Je lokale bestanden ({ $local-path }) worden dan een exacte kopie van je cloudbestanden ({ $cloud-path }).
    Lokale bestanden worden bijgewerkt of verwijderd waar nodig.
confirm-add-missing-roots = Voeg deze roots toe?
no-missing-roots = Geen extra roots gevonden.
loading = Bezig met laden...
preparing-backup-target = Back-upmap wordt voorbereid...
updating-manifest = Manifest wordt bijgewerkt...
no-cloud-changes = Geen veranderingen om te synchroniseren
backups-are-valid = Je back-ups zijn ongeldig.
backups-are-invalid =
    De back-ups van deze spellen lijken ongeldig.
    Wil je nieuwe volledige back-ups maken voor deze spellen?
saves-found = Savedata gevonden.
no-saves-found = Geen savedata gevonden.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = geen bevestiging
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = opnieuw opstarten vereist
prefix-error = Error: { $message }
prefix-warning = Waarschuwing: { $message }
cloud-app-unavailable = Cloudback-ups zijn uitgeschakeld omdat { $app } niet beschikbaar is.
cloud-not-configured = Cloudback-ups zijn uitgeschakeld omdat er geen cloudsysteem is geconfigureerd.
cloud-path-invalid = Cloudback-ups zijn uitgeschakeld omdat het back-uppad ongeldig is.
game-is-unrecognized = Ludasavi herkent dit spel niet.
game-has-nothing-to-restore = Dit spel heeft geen back-up om te herstellen.
launch-game-after-error = Moet het spel toch gestart worden?
game-did-not-launch = Het spel kon niet gestart worden.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Savedata voor { $game } back-uppen?
    .failed = Kon geen back-up maken van savedata voor { $game }
restore-specific-game =
    .confirm = Herstel savedata voor { $game }?
    .failed = Savedata voor { $game } kon niet hersteld worden
new-version-check = Controleer automatisch op applicatie-updates
new-version-available = Een applicatie-update is beschikbaar: { $version }. Wil je de uitgaveopmerkingen bekijken?
custom-game-will-override = This custom game overrides a manifest entry
custom-game-will-extend = This custom game extends a manifest entry
operation-will-only-include-listed-games = This will only process the games that are currently listed

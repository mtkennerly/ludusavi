ludusavi = Ludusavi
language = Sprache
game-name = Name
total-games = Spiele
file-size = Größe
file-location = Ort
overall = Insgesamt
status = Status
cli-unrecognized-games = Keine Informationen für diese Spiele:
cli-unable-to-request-confirmation = Bestätigung konnte nicht angefordert werden.
    .winpty-workaround = Falls du einen Bash-Emulator (wie Git Bash) verwendest, versuche winpty auszuführen.
cli-backup-id-with-multiple-games = Sicherungs-ID kann nicht angegeben werden, wenn mehrere Spiele wiederhergestellt werden.
cli-invalid-backup-id = Ungültige Sicherungs-ID.
badge-failed = FEHLGESCHLAGEN
badge-duplicates = DUPLIKATE
badge-duplicated = DUPLIZIERT
badge-ignored = IGNORIERT
badge-redirected-from = VON: { $path }
badge-redirecting-to = AN: { $path }
some-entries-failed = Einige Einträge konnten nicht verarbeitet werden. Suche innerhalb der Ausgabe nach { badge-failed } für Details. Überprüfe, ob du auf diese Dateien zugreifen kannst oder ob deren Pfade sehr lang sind.
cli-game-line-item-redirected = Umgeleitet von: { $path }
cli-game-line-item-redirecting = Umleiten an: { $path }
button-backup = Sichern
button-preview = Vorschau
button-restore = Wiederherstellen
button-nav-backup = SICHERUNGSMODUS
button-nav-restore = WIEDERHERSTELLUNGSMODUS
button-nav-custom-games = BENUTZERDEFINIERTE SPIELE
button-nav-other = ANDERE
button-add-game = Spiel hinzufügen
button-continue = Weiter
button-cancel = Abbrechen
button-cancelling = Abbrechen...
button-okay = Okay
button-select-all = Alle auswählen
button-deselect-all = Alle abwählen
button-enable-all = Alle aktivieren
button-disable-all = Alle deaktivieren
button-customize = Anpassen
button-exit = Verlassen
button-comment = Kommentieren
button-lock = Sperren
button-unlock = Entsperren
# This opens a download page.
button-get-app = { $app } holen
button-validate = Überprüfen
button-override-manifest = Manifest überschreiben
button-extend-manifest = Manifest erweitern
button-sort = Sortieren
button-download = Herunterladen
button-upload = Hochladen
button-ignore = Ignorieren
no-roots-are-configured = Füge einige Wurzelverzeichnisse hinzu, um weitere Daten zu sichern.
config-is-invalid = Fehler: Die Konfigurationsdatei ist ungültig.
manifest-is-invalid = Fehler: Die Manifest-Datei ist ungültig.
manifest-cannot-be-updated = Fehler: Die Manifest-Datei konnte nicht auf eine Aktualisierung überprüft werden. Besteht eine Internetverbindung?
cannot-prepare-backup-target = Fehler: Das Sicherungsziel kann nicht vorbereitet werden (entweder beim Erstellen oder Leeren des Ordners). Falls du den Ordner in deinem Dateibrowser geöffnet hast, versuche diesen zu schließen: { $path }
restoration-source-is-invalid = Fehler: Die Wiederherstellungsquelle ist ungültig (entweder sie existiert nicht oder ist kein Verzeichnis). Bitte überprüfe den Speicherort: { $path }
registry-issue = Fehler: Einige Registrierungseinträge wurden übersprungen.
unable-to-browse-file-system = Fehler: Dateisystem kann nicht durchsucht werden.
unable-to-open-directory = Fehler: Verzeichnis konnte nicht geöffnet werden:
unable-to-open-url = Fehler: Kann URL nicht öffnen:
unable-to-configure-cloud = Cloud konnte nicht konfiguriert werden.
unable-to-synchronize-with-cloud = Cloud konnte nicht synchronisiert werden.
cloud-synchronize-conflict = Deine lokalen und Cloud-Backups stehen im Konflikt. Führe einen Upload oder Download durch, um das Problem zu lösen.
command-unlaunched = Befehl wurde nicht gestartet: { $command }
command-terminated = Befehl wurde abrupt beendet: { $command }
command-failed = Befehl fehlgeschlagen mit Code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] Spiel
       *[other] Spiele
    }
processed-games-subset =
    { $processed-games } von { $total-games } { $total-games ->
        [one] Spiel
       *[other] Spiele
    }
processed-size-subset = { $processed-size } von { $total-size }
field-backup-target = Sichern nach:
field-restore-source = Wiederherstellen von:
field-custom-files = Pfade:
field-custom-registry = Registry:
field-sort = Sortierung:
field-redirect-source =
    .placeholder = Quelle (Originalort)
field-redirect-target =
    .placeholder = Ziel (neuer Ort)
field-roots = Wurzelverzeichnisse:
field-backup-excluded-items = Sicherungsausschlüsse:
field-redirects = Umleitungen:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Komplett:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differenz:
field-backup-format = Format:
field-backup-compression = Komprimierung:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Stufe:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Überprüft
# This shows the time when we found an update to the manifest.
label-updated = Aktualisiert
label-new = Neu
label-removed = Entfernt
label-comment = Kommentar
label-unchanged = Unverändert
label-backup = Sicherung
label-scan = Scan
label-filter = Filter
label-unique = Einzigartig
label-complete = Vollständig
label-partial = Teilweise
label-enabled = Aktiviert
label-disabled = Deaktiviert
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remote
label-remote-name = Remote-Name
label-folder = Ordner
# An executable file
label-executable = Ausführbare Datei
# Options given to a command line program
label-arguments = Argumente
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Port
label-username = Nutzername
label-password = Passwort
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Provider
label-custom = Benutzerdefiniert
label-none = Keiner
label-change-count = Änderungen: { $total }
label-unscanned = Ungescannt
# This refers to a local file on the computer
label-file = Datei
label-game = Spiel
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Originalname
# Which manifest a game's data came from
label-source = Quelle
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primäres Manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integration
# This is a folder name where a specific game is installed
label-installed-name = Installation
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
store-other-home = Home-Ordner
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine-Präfix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows-Laufwerk
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux-Laufwerk
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac-Laufwerk
store-other = Sonstiges
backup-format-simple = Einfach
backup-format-zip = Zip
compression-none = Keiner
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Aussehen
theme-light = Hell
theme-dark = Dunkel
redirect-bidirectional = Bidirektional
reverse-redirects-when-restoring = Beim Wiederherstellen die Reihenfolge der Umleitungen umkehren
show-disabled-games = Deaktivierte Spiele anzeigen
show-unchanged-games = Unveränderte Spiele anzeigen
show-unscanned-games = Ungescannte Spiele anzeigen
override-max-threads = Max. Threads übergehen
synchronize-automatically = Automatisch synchronisieren
prefer-alias-display = Alias statt Originalnamen anzeigen
skip-unconstructive-backups = Backup überspringen, wenn nur Daten entfernt werden würden, anstatt hinzugefügt oder geändert zu werden
explanation-for-exclude-store-screenshots = Storespezifische Bildschirmfotos aus Datensicherungen ausschießen
explanation-for-exclude-cloud-games = Auf diesen Plattformen keine Spiele mit Cloud-Unterstützung sichern
consider-doing-a-preview =
    Falls du es noch nicht getan hast, erwäge zuerst eine Vorschau zu machen, damit
    keine Überraschungen gibt.
confirm-backup =
    Bist du sicher, dass du mit der Sicherung fortfahren möchtest? { $path-action ->
        [merge] Neue Spielstanddaten werden mit dem Zielordner zusammengeführt:
       *[create] Der Zielordner wird erstellt:
    }
confirm-restore =
    Bist du sicher, dass du mit der Wiederherstellung fortfahren möchtest?
    Dies überschreibt alle aktuellen Dateien mit den Sicherungen von hier:
confirm-cloud-upload =
    Möchtest du deine Cloud-Dateien mit deinen lokalen Dateien ersetzen?
    Deine Cloud-Dateien ({ $cloud-path }) werden zu einer exakten Kopie deiner lokalen Dateien ({ $local-path }).
    Dateien in der Cloud werden bei Bedarf aktualisiert oder gelöscht.
confirm-cloud-download =
    Möchtest du deine lokalen Dateien mit deinen Cloud-Dateien ersetzen?
    Deine lokalen Dateien ({ $local-path }) werden zu einer exakten Kopie deiner Cloud-Dateien ({ $cloud-path }).
    Lokale Dateien werden bei Bedarf aktualisiert oder gelöscht.
confirm-add-missing-roots = Diese Wurzelverzeichnisse hinzufügen?
no-missing-roots = Keine weiteren Wurzelverzeichnisse gefunden.
loading = Lädt …
preparing-backup-target = Sicherungsverzeichnis wird vorbereitet...
updating-manifest = Manifest wird aktualisiert...
no-cloud-changes = Keine zu sychronisierenden Änderungen
backups-are-valid = Deine Sicherungen sind gültig.
backups-are-invalid =
    Die Sicherungen dieser Spiele scheinen ungültig zu sein.
    Möchtest du für diese Spiele neue vollständige Sicherungen erstellen?
saves-found = Spielstanddaten gefunden.
no-saves-found = Keine Spielstanddaten gefunden.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = ohne Bestätigung
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = Neustart erforderlich
prefix-error = Fehler: { $message }
prefix-warning = Warnung: { $message }
cloud-app-unavailable = Cloud-Backups sind deaktiviert, da { $app } nicht verfügbar ist.
cloud-not-configured = Cloud-Backups sind deaktiviert, da kein Cloudsystem konfiguriert ist.
cloud-path-invalid = Cloud-Backups sind deaktiviert, da der Backup-Pfad ungültig ist.
game-is-unrecognized = Ludusavi erkennt dieses Spielt nicht.
game-has-nothing-to-restore = Dieses Spiel hat keine wiederherzustellende Sicherungskopie.
launch-game-after-error = Spiel trotzdem starten?
game-did-not-launch = Spiel konnte nicht gestartet werden.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Spielstanddaten für { $game } sichern?
    .failed = Spielstanddaten für { $game } konnten nicht gesichert werden.
restore-specific-game =
    .confirm = Spielstanddaten für { $game } wiederherstellen?
    .failed = Spielstanddaten für { $game } konnten nicht wiederhergestellt werden.
new-version-check = Automatisch nach Aktualisierungen der Anwendung suchen
new-version-available = Eine Anwendungsaktualisierung ist verfügbar: { $version }. Möchtest du die Versionshinweise ansehen?
custom-game-will-override = Dieses benutzerdefinierte Spiel überschreibt einen Manifest-Eintrag
custom-game-will-extend = Dieses benutzerdefinierte Spiel erweitert einen Manifest-Eintrag
operation-will-only-include-listed-games = Hiermit werden nur die derzeit aufgelisteten Spiele verarbeitet

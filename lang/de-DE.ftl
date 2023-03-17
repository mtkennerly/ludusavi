ludusavi = Ludusavi
language = Sprache
font = Schriftart
game-name = Name
total-games = Spiele
file-size = Größe
file-location = Ort
overall = Insgesamt
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
toggle-backup-merge = Vereinen
field-restore-source = Wiederherstellen von:
field-custom-files = Pfade:
field-custom-registry = Registry:
field-search = Suche:
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
label-comment = Kommentar
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home-Ordner
store-other-wine = Wine prefix
store-other = Sonstiges
sort-reversed = Umgekehrt
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
explanation-for-exclude-store-screenshots = Schließe Starterspezifische Bildschirmfotos in Sicherungen aus. Dies gilt momentan nur für { store-steam }-Bildschirmfotos. Wenn ein Spiel eine eigene Bildschirmfotofunktion hat, werden Bildschirmfotos unabhängig dieser Einstellung gesichert.
consider-doing-a-preview =
    Falls du es noch nicht getan hast, erwäge zuerst eine Vorschau zu machen, damit
    keine Überraschungen gibt.
confirm-backup =
    Bist du sicher, dass du mit der Sicherung fortfahren möchtest? { $path-action ->
        [merge] Neue Speicherdaten werden mit dem Zielordner zusammengeführt:
        [recreate] Der Zielordner wird gelöscht und von Grund auf neu erstellt:
       *[create] Der Zielordner wird erstellt:
    }
confirm-restore =
    Bist du sicher, dass du mit der Wiederherstellung fortfahren möchtest?
    Dies überschreibt alle aktuellen Dateien mit den Sicherungen von hier:
confirm-add-missing-roots = Diese Wurzelverzeichnisse hinzufügen?
no-missing-roots = Keine weiteren Wurzelverzeichnisse gefunden.
preparing-backup-target = Sicherungsverzeichnis wird vorbereitet...
updating-manifest = Manifest wird aktualisiert...
saves-found = Spielstanddaten gefunden.
no-saves-found = Keine Spielstanddaten gefunden.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = ohne Bestätigung

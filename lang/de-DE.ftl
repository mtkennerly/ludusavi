ludusavi = Ludusavi
language = Sprache
font = Schriftart
game-name = Name
total-games = Spiele
file-size = Größe
file-location = Ort
overall = Insgesamt
cli-backup-target-already-exists = Das Sicherungsziel exitiert bereits ( { $path } ). Wähle einen anderen Ort mit --path oder lösche ihn mit --force.
cli-unrecognized-games = Keine Informationen für diese Spiele:
cli-confirm-restoration = Wollen Sie von { $path } wiederherstellen?
cli-unable-to-request-confirmation = Bestätigung konnte nicht angefordert werden.
    .winpty-workaround = Wenn du einen Bash Emulator (wie Git Bash) verwendest, versuche winpty auszuführen.
badge-failed = FEHLGESCHLAGEN
badge-duplicates = DUPLIKATE
badge-duplicated = DUPLIZIERT
badge-ignored = IGNORIERT
badge-redirected-from = VON: { $path }
some-entries-failed = Einige Einträge konnten nicht verarbeitet werden; Suche nach { badge-failed } in der Ausgabe für Details. Überprüfen Sie, ob Sie auf diese Dateien zugreifen können oder ob ihre Pfade sehr lang sind.
cli-game-line-item-redirected = Umgeleitet von: { $path }
button-backup = Sichern
button-preview = Vorschau
button-restore = Wiederherstellen
button-nav-backup = SICHERUNGSMODUS
button-nav-restore = WIEDERHERSTELLUNGSMODUS
button-nav-custom-games = BENUTZERDEFINIERTE SPIELE
button-nav-other = ANDERE
button-add-root = Wurzel hinzufügen
button-find-roots = Wurzeln suchen
button-add-redirect = Weiterleitung hinzufügen
button-add-game = Spiel hinzufügen
button-continue = Weiter
button-cancel = Abbrechen
button-cancelling = Abbrechen...
button-okay = Okay
button-select-all = Alle auswählen
button-deselect-all = Alle abwählen
button-enable-all = Alle aktivieren
button-disable-all = Alle deaktivieren
no-roots-are-configured = Fügen Sie einige Wurzeln hinzu, um noch mehr Daten zu sichern.
config-is-invalid = Fehler: Die Konfigurationsdatei ist ungültig.
manifest-is-invalid = Fehler: Die Manifest-Datei ist ungültig.
manifest-cannot-be-updated = Fehler: Konnte nicht auf eine Aktualisierung der Manifest-Datei überprüfen. Ist Ihre Internetverbindung abgeschlossen?
cannot-prepare-backup-target = Fehler: Kann Backup-Ziel nicht vorbereiten (entweder erstellen oder leeren des Ordners). Wenn Sie den Ordner in Ihrem Datei-Browser geöffnet haben, schließen Sie ihn: { $path }
restoration-source-is-invalid = Fehler: Die Wiederherstellungsquelle ist ungültig (entweder existiert nicht oder ist kein Verzeichnis). Bitte überprüfen Sie den Speicherort: { $path }
registry-issue = Fehler: Einige Registrierungseinträge wurden übersprungen.
unable-to-browse-file-system = Fehler: Konnte ihr System nicht durchsuchen.
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
field-backup-excluded-items = Sicherungsausschlüsse:
field-retention-full = Komplett:
field-retention-differential = Differenz:
field-backup-format = Format:
field-backup-compression = Komprimierung:
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-gog-linux = GOG Linux
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home-Ordner
store-other-wine = Wine prefix
store-other = Sonstiges
sort-name = Name
sort-size = Größe
sort-reversed = Umgekehrt
backup-format-simple = Einfach
backup-format-zip = Zip
compression-none = Keiner
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
explanation-for-exclude-other-os-data =
    Ausschließen von Speicherorten, die nur auf einem anderen
    Betriebssystem bestätigt wurden. Einige Spiele stellen immer Speicherstände an derselben Stelle, aber die
    Standorte sind möglicherweise nur für ein anderes Betriebssystem bestätigt worden, also kann es
    helfen, sie trotzdem zu überprüfen. Das Ausschließen dieser Daten kann helfen, Fehlerhafte Einträge zu vermeiden,
    kann aber auch bedeuten, dass einige Speicherstände nicht mehr vorhanden sind. Unter Linux werden ungeachtet dieser Einstellung weiterhin Proton speicherstände
    gesichert.
explanation-for-exclude-store-screenshots = Schließe Starterspezifische Bildschirmfotos in Sicherungen aus. Dies gilt momentan nur für { store-steam }-Bildschirmfotos. Wenn ein Spiel eine eigene Bildschirmfotofunktion hat, werden Bildschirmfotos unabhängig dieser Einstellung gesichert.
consider-doing-a-preview =
    Falls du es noch nicht getan hast, erwäge zuerst eine Vorschau zu machen, damit
    keine Überraschungen gibt.
confirm-backup =
    Sind Sie sicher, dass Sie die Sicherung fortsetzen möchten? { $path-action ->
        [merge] Neue Speicherdaten werden in den Zielordner zusammengeführt:
        [recreate] Der Zielordner wird gelöscht und von Grund auf neu erstellt:
       *[create] Der Zielordner wird erstellt:
    }
confirm-restore =
    Sind Sie sicher, dass Sie die Wiederherstellung fortsetzen möchten?
    Dies überschreibt alle aktuellen Dateien mit den Sicherungen von hier:
confirm-add-missing-roots = Diese Wurzel hinzufügen?
no-missing-roots = Keine zusätzlichen Wurzeln gefunden.
preparing-backup-target = Sicherungsverzeichnis wird vorbereitet...

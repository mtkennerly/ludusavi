ludusavi = Ludusavi
language = Język
game-name = Nazwa
total-games = Gry
file-size = Rozmiar
file-location = Lokalizacja
overall = W sumie
status = Status
cli-unrecognized-games = Brak informacji dla tych gier:
cli-unable-to-request-confirmation = Błąd żądania potwierdzenia.
    .winpty-workaround = Jeśli korzystasz z emulatora Bash (takiego jak Git Bash), spróbuj uruchomić winpty.
cli-backup-id-with-multiple-games = Nie można określić identyfikatora kopii zapasowej podczas przywracania wielu gier.
cli-invalid-backup-id = Nieprawidłowy identyfikator kopii zapasowej.
badge-failed = NIEPOWODZENIE
badge-duplicates = DUPLIKATY
badge-duplicated = ZDUPLIKOWANE
badge-ignored = ZIGNOROWANE
badge-redirected-from = Z: { $path }
badge-redirecting-to = DO: { $path }
some-entries-failed = Błąd przetwarzania niektórych elementów; sprawdź { badge-failed } w anych wyjściowych po więcej szczegółów. Upewnij się, że masz dostęp do tych plików oraz, czy ich ścieżki są zbyt długie.
cli-game-line-item-redirected = Przekierowano z: { $path }
cli-game-line-item-redirecting = Przekierowywanie do: { $path }
button-backup = Utwórz kopię
button-preview = Podgląd
button-restore = Przywróć
button-nav-backup = TRYB TWORZENIA KOPII
button-nav-restore = TRYB PRZYWRACANIA
button-nav-custom-games = NIESTANDARDOWE GRY
button-nav-other = POZOSTAŁE
button-add-game = Dodaj grę
button-continue = Kontynuuj
button-cancel = Anuluj
button-cancelling = Anulowanie...
button-okay = OK
button-select-all = Zaznacz wszystkie
button-deselect-all = Odznacz wszystkie
button-enable-all = Włącz wszystkie
button-disable-all = Wyłącz wszystkie
button-customize = Dostosuj
button-exit = Wyjdź
button-comment = Komentarz
# This opens a download page.
button-get-app = Get { $app }
no-roots-are-configured = Dodaj kilka katalogów głównych, aby utworzyć kopię większej ilości danych.
config-is-invalid = Błąd: Plik konfiguracji jest nieprawidłowy.
manifest-is-invalid = Błąd: Plik manifest jest nieprawidłowy.
manifest-cannot-be-updated = Błąd: Nie można sprawdzić aktualizacji dla pliku manifest. Czy masz połączenie z Internetem?
cannot-prepare-backup-target = Błąd: Nie można przygotować docelowej kopii zapasowej (utworzyć lub oczyścić folderu). Jeśli folder jest otwarty w eksploratorze plików, zamknij go: { $path }
restoration-source-is-invalid = Błąd: Źródło przywracania jest nieprawidłowe (nie istnieje lub nie jest katalogiem) Upewnij się, że lokalizacja jest prawidłowa: { $path }
registry-issue = Błąd: Niektóre pozycje rejestru zostały pominięte.
unable-to-browse-file-system = Błąd. Nie można przeglądać na Twoim systemie.
unable-to-open-directory = Błąd: Nie można otworzyć katalogu:
unable-to-open-url = Błąd: Nie można otworzyć adresu URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] gra
       *[other] gier
    }
processed-games-subset =
    { $processed-games } z { $total-games } { $total-games ->
        [one] gra
       *[other] gier
    }
processed-size-subset = { $processed-size } z { $total-size }
field-backup-target = Utwórz kopię w:
field-restore-source = Przywróć z:
field-custom-files = Ścieżki:
field-custom-registry = Rejestr:
field-sort = Sortuj:
field-redirect-source =
    .placeholder = Źródło (oryginalna lokalizacja)
field-redirect-target =
    .placeholder = Cel (nowa lokalizacja)
field-roots = Źródło:
field-backup-excluded-items = Wykluczenia kopii zapasowych:
field-redirects = Przekierowania:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Pełne:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Różnicowe:
field-backup-format = Format:
field-backup-compression = Kompresja:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Poziom kompresji:
label-manifest = Wzory ścieżek zapisu
# This shows the time when we checked for an update to the manifest.
label-checked = Sprawdzono
# This shows the time when we found an update to the manifest.
label-updated = Zaktualizowano
label-new = Nowy
label-removed = Removed
label-comment = Komentarz
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
label-provider = Provider
label-custom = Custom
label-none = None
label-change-count = Changes: { $total }
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Folder główny
store-other-wine = Prefiks Wine
store-other = Pozostałe
backup-format-simple = Prosty
backup-format-zip = Zip
compression-none = Brak
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Motyw
theme-light = Jasny
theme-dark = Ciemny
redirect-bidirectional = Dwukierunkowy
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots = Nie zawieraj w kopiach zapasowych zrzutów ekranu dla konkretnego sklepu
consider-doing-a-preview = Jeśli jeszcze tego nie zrobiono, rozważ wykonanie pierwszego testu, aby zobaczyć, czy wszystko działa.
confirm-backup =
    Czy na pewno chcesz kontynuować z kopią zapasową? { $path-action ->
        [merge] Nowe dane zapisu zostaną scalone z folderem docelowym:
       *[create] Folder docelowy zostanie utworzony:
    }
confirm-restore =
    Czy na pewno chcesz kontynuować przywracanie?
    Jakiekolwiek bieżące pliki z kopią zapasową zostaną zastąpione:
confirm-cloud-upload =
    Do you want to synchronize your local files to the cloud?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to synchronize your cloud files to this system?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Czy to są katalogi główne?
no-missing-roots = Nie znaleziono więcej katalogów głównych.
loading = Loading...
preparing-backup-target = Przygotowywanie katalogu kopii zapasowej...
updating-manifest = Aktualizowanie manifestu...
no-cloud-changes = No changes to synchronize
saves-found = Znaleziono dane zapisu.
no-saves-found = Nie znaleziono danych zapisu.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = bez potwierdzenia
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

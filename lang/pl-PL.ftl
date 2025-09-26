ludusavi = Ludusavi
language = Język
game-name = Nazwa
total-games = Gry
file-size = Rozmiar
file-location = Lokalizacja
overall = Całościowo
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
button-lock = Zablokuj
button-unlock = Odblokuj
# This opens a download page.
button-get-app = Pobierz { $app }
button-validate = Zweryfikuj
button-override-manifest = Nadpisanie manifestu
button-extend-manifest = Rozszerzenie manifestu
button-sort = Sortowanie
button-download = Pobierz
button-upload = Wyślij
button-ignore = Ignoruj
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
unable-to-configure-cloud = Nie udało się skonfigurować chmury.
unable-to-synchronize-with-cloud = Nie można zsynchronizować z chmurą.
cloud-synchronize-conflict = Kopia lokalna różni się od tej w chmurze. Wyślij lub pobierz odpowiednią wersję, aby rozwiązać problem.
command-unlaunched = Polecenia nie uruchomiono: { $command }
command-terminated = Polecenie zakończone nagle: { $command }
command-failed = Polecenie nie powiodło się z kodem { $code }: { $command }
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
label-removed = Usunięto
label-comment = Komentarz
label-unchanged = Bez zmian
label-backup = Kopia zapasowa
label-scan = Skan
label-filter = Filtruj
label-unique = Unikalne
label-complete = Pełny
label-partial = Częściowy
label-enabled = Aktywny
label-disabled = Nieaktywny
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Wątki
label-cloud = Chmura
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Zdalny
label-remote-name = Nazwa zdalnego
label-folder = Folder
# An executable file
label-executable = Plik wykonywalny
# Options given to a command line program
label-arguments = Parametry
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Port
label-username = Nazwa użytkownika
label-password = Hasło
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Usługodawca
label-custom = Własny
label-none = Brak
label-change-count = Zmiany: { $total }
label-unscanned = Nieprzeskanowane
# This refers to a local file on the computer
label-file = Plik
label-game = Gra
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Oryginalna nazwa
# Which manifest a game's data came from
label-source = Źródło
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Główny manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integracja
# This is a folder name where a specific game is installed
label-installed-name = Nazwa instalacji
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
store-other-home = Folder główny
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Prefiks Wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Dysk Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Dysk Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Dysk Mac
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
reverse-redirects-when-restoring = Odwrotna sekwencja przekierowań podczas przywracania
show-disabled-games = Pokaż wyłączone gry
show-unchanged-games = Pokaż niezmienione gry
show-unscanned-games = Pokaż nieprzeskanowane gry
override-max-threads = Zastąp maksymalną liczbę wątków
synchronize-automatically = Synchronizuj automatycznie
prefer-alias-display = Wyświetlaj alias zamiast oryginalnej nazwy
skip-unconstructive-backups = Pomiń kopię zapasową, gdy dane zostaną usunięte, ale nie dodane lub zaktualizowane
explanation-for-exclude-store-screenshots = Nie zawieraj w kopiach zapasowych zrzutów ekranu dla konkretnego sklepu
explanation-for-exclude-cloud-games = Na tych platformach nie należy tworzyć kopii zapasowych gier z obsługą chmury
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
    Czy chcesz zastąpić pliki w chmurze plikami lokalnymi?
    Twoje pliki ({ $cloud-path }) staną się dokładną kopią plików lokalnych ({ $local-path }).
    Pliki w chmurze zostaną w razie potrzeby zaktualizowane lub usunięte.
confirm-cloud-download =
    Czy chcesz zastąpić pliki lokalne plikami w chmurze?
    Twoje lokalne pliki ({ $local-path }) staną się dokładną kopią Twoich plików w chmurze ({ $cloud-path }).
    Pliki lokalne zostaną w razie potrzeby zaktualizowane lub usunięte.
confirm-add-missing-roots = Czy to są katalogi główne?
no-missing-roots = Nie znaleziono więcej katalogów głównych.
loading = Ładowanie...
preparing-backup-target = Przygotowywanie katalogu kopii zapasowej...
updating-manifest = Aktualizowanie manifestu...
no-cloud-changes = Nie ma zmian do synchronizacji
backups-are-valid = Twoje kopie zapasowe są prawidłowe.
backups-are-invalid =
    Kopie zapasowe tych gier wydają się nieprawidłowe.
    Czy chcesz utworzyć nowe pełne kopie zapasowe?
saves-found = Znaleziono dane zapisu.
no-saves-found = Nie znaleziono danych zapisu.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = bez potwierdzenia
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = wymagane ponowne uruchomienie
prefix-error = Błąd: { $message }
prefix-warning = Ostrzeżenie: { $message }
cloud-app-unavailable = Kopie zapasowe w chmurze są wyłączone, ponieważ { $app } jest niedostępny.
cloud-not-configured = Kopie zapasowe w chmurze są wyłączone, ponieważ nie skonfigurowano żadnego systemu w chmurze.
cloud-path-invalid = Kopie zapasowe w chmurze są wyłączone, ponieważ ścieżka kopii zapasowej jest nieprawidłowa.
game-is-unrecognized = Ludusavi nie rozpoznaje tej gry.
game-has-nothing-to-restore = Ta gra nie ma kopii zapasowej do przywrócenia.
launch-game-after-error = Czy mimo to uruchomić grę?
game-did-not-launch = Nie udało się uruchomić gry.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Stworzyć kopię zapisów dla { $game }?
    .failed = Nie udało się utworzyć kopii zapisów dla { $game }
restore-specific-game =
    .confirm = Przywrócić zapisy dla { $game }?
    .failed = Nie udało się przywrócić zapisów dla { $game }
new-version-check = Automatyczne sprawdzanie aktualizacji aplikacji
new-version-available = Dostępna jest aktualizacja aplikacji: { $version }. Chcesz zobaczyć informacje o wydaniu?
custom-game-will-override = Ta niestandardowa gra zastępuje wpis manifestu
custom-game-will-extend = Ta niestandardowa gra rozszerza wpis manifestu
operation-will-only-include-listed-games = Spowoduje to przetworzenie tylko tych gier, które aktualnie znajdują się na liście

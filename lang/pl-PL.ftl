ludusavi = Ludusavi
language = Język
font = Czcionka
game-name = Nazwa
total-games = Gry
file-size = Rozmiar
file-location = Lokalizacja
overall = W sumie
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
badge-redirecting-to = TO: { $path }
some-entries-failed = Błąd przetwarzania niektórych elementów; sprawdź { badge-failed } w anych wyjściowych po więcej szczegółów. Upewnij się, że masz dostęp do tych plików oraz, czy ich ścieżki są zbyt długie.
cli-game-line-item-redirected = Przekierowano z: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
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
button-customize = Customize
button-exit = Exit
button-comment = Comment
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
toggle-backup-merge = Scal
field-restore-source = Przywróć z:
field-custom-files = Ścieżki:
field-custom-registry = Rejestr:
field-search = Szukaj:
field-sort = Sort:
field-redirect-source =
    .placeholder = Źródło (oryginalna lokalizacja)
field-redirect-target =
    .placeholder = Cel (nowa lokalizacja)
field-roots = Roots:
field-backup-excluded-items = Wykluczenia kopii zapasowych:
field-redirects = Redirects:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Pełne:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Różnicowe:
field-backup-format = Format:
field-backup-compression = Kompresja:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Level:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Checked
# This shows the time when we found an update to the manifest.
label-updated = Updated
label-new = New
label-comment = Comment
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
sort-reversed = Odwrotne
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
redirect-bidirectional = Bidirectional
explanation-for-exclude-store-screenshots = Nie zawieraj w kopiach zapasowych zrzutów ekranu dla konkretnego sklepu. Obecnie ma to tylko zastosowanie do zrzutów ekranu ze { store-steam }. Jeśli gra ma swoją własną funkcję zrzutów ekranu, to ustawienie nie będzie mieć wpływu na ich kopiowanie.
consider-doing-a-preview = Jeśli jeszcze tego nie zrobiono, rozważ wykonanie pierwszego testu, aby zobaczyć, czy wszystko działa.
confirm-backup =
    Czy na pewno chcesz kontynuować z kopią zapasową? { $path-action ->
        [merge] Nowe dane zapisu zostaną scalone z folderem docelowym:
        [recreate] Folder docelowy zostanie usunięty i odtworzony od zera:
       *[create] Folder docelowy zostanie utworzony:
    }
confirm-restore =
    Czy na pewno chcesz kontynuować przywracanie?
    Jakiekolwiek bieżące pliki z kopią zapasową zostaną zastąpione:
confirm-add-missing-roots = Czy to są katalogi główne?
no-missing-roots = Nie znaleziono więcej katalogów głównych.
preparing-backup-target = Przygotowywanie katalogu kopii zapasowej...
updating-manifest = Aktualizowanie manifestu...
saves-found = Save data found.
no-saves-found = No save data found.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = no confirmation

ludusavi = Ludusavi
language = Język
language-font-compatibility = Niektóre języki mogą wymagać niestandardowej czcionki.
font = Czcionka
cli-backup-target-already-exists = Docelowa kopia zapasowa już istnieje ( { $path } ). Wybierz inną ścieżkę --path lub usuń kopię poprzez --force.
cli-unrecognized-games = Brak informacji dla tych gier:
cli-confirm-restoration = Czy chcesz przywrócić z { $path }?
cli-unable-to-request-confirmation = Błąd żądania potwierdzenia.
    .winpty-workaround = Jeśli korzystasz z emulatora Bash (takiego jak Git Bash), spróbuj uruchomić winpty.
badge-failed = NIEPOWODZENIE
badge-duplicates = DUPLIKATY
badge-duplicated = ZDUPLIKOWANE
badge-ignored = ZIGNOROWANE
badge-redirected-from = Z: { $path }
some-entries-failed = Błąd przetwarzania niektórych elementów; sprawdź { label-failed } w anych wyjściowych po więcej szczegółów. Upewnij się, że masz dostęp do tych plików oraz, czy ich ścieżki są zbyt długie.
cli-game-line-item-redirected = Przekierowano z: { $path }
cli-summary =
    .succeeded =
        W sumie:
          Gry: { $total-games }
          Rozmiar: { $total-size }
          Lokalizacja: { $path }
    .failed =
        W sumie:
          Gry: { $processed-games } z { $total-games }
          Rozmiar: { $processed-size } z { $total-size }
          Lokalizacja: { $path }
button-backup = Utwórz kopię
button-preview = Podgląd
button-restore = Przywróć
button-nav-backup = TRYB TWORZENIA KOPII
button-nav-restore = TRYB PRZYWRACANIA
button-nav-custom-games = NIESTANDARDOWE GRY
button-nav-other = POZOSTAŁE
button-add-root = Dodaj katalog główny
button-find-roots = Znajdź katalogi główne
button-add-redirect = Dodaj przekierowanie
button-add-game = Dodaj grę
button-continue = Kontynuuj
button-cancel = Anuluj
button-cancelling = Anulowanie...
button-okay = OK
button-select-all = Zaznacz wszystkie
button-deselect-all = Odznacz wszystkie
button-enable-all = Włącz wszystkie
button-disable-all = Wyłącz wszystkie
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
field-custom-game-name =
    .placeholder = Nazwa
field-search-game-name =
    .placeholder = Nazwa
field-backup-excluded-items = Backup exclusions:
field-retention-full = Full:
field-retention-differential = Differential:
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Folder główny
store-other-wine = Prefiks Wine
store-other = Pozostałe
sort-name = Name
sort-size = Size
sort-reversed = Reversed
explanation-for-exclude-other-os-data = Nie zawieraj w kopiach zapasowych lokalizacji zapisów, które zostały sprawdzone tylko na innym systemie. Niektóre gry zawsze umieszczają zapisy w tym samym miejscu, ale lokalizacje mogą być sprawdzane tylko dla innego systemu operacyjnego, więc możesz je sprawdzić mimo wszystko. Wykluczenie tych danych może pomóc uniknąć fałszywego wykrycia zagrożeń, ale może również wykluczyć niektóre zapisy. W systemach Linux, zapisy Proton będą nadal kopiowane bez względu na to ustawienie.
explanation-for-exclude-store-screenshots = Nie zawieraj w kopiach zapasowych zrzutów ekranu dla konkretnego sklepu. Obecnie ma to tylko zastosowanie do zrzutów ekranu ze { store-steam }. Jeśli gra ma swoją własną funkcję zrzutów ekranu, to ustawienie nie będzie mieć wpływu na ich kopiowanie.
consider-doing-a-preview = Jeśli jeszcze tego nie zrobiono, rozważ wykonanie pierwszego testu, aby zobaczyć, czy wszystko działa.
confirm-backup =
    Czy na pewno chcesz kontynuować z kopią zapasową? { $path-action ->
        [merge] Nowe dane zapisu zostaną scalone z folderem docelowym
        [recreate] Folder docelowy zostanie usunięty i odtworzony od zera
       *[create] Folder docelowy zostanie utworzony
    }:

    { $path }

    { consider-doing-a-preview }
confirm-restore =
    Czy na pewno chcesz kontynuować przywracanie?
    Jakiekolwiek bieżące pliki z kopią zapasową zostaną zastąpione:

    { $path }

    { consider-doing-a-preview }
confirm-add-missing-roots = Czy to są katalogi główne?
no-missing-roots = Nie znaleziono więcej katalogów głównych.
preparing-backup-target = Preparing backup directory...

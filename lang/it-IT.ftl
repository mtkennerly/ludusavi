ludusavi = Ludusavi
language = Lingua
font = Font
game-name = Nome
total-games = Giochi
file-size = Dimensione
file-location = Posizione
overall = Nel complesso
cli-unrecognized-games = Nessuna informazione per questi giochi:
cli-unable-to-request-confirmation = Impossibile richiedere conferma.
    .winpty-workaround = Se stai usando un emulatore Bash (come Git Bash), prova ad eseguire winpty.
cli-backup-id-with-multiple-games = Impossibile specificare l'ID di backup quando si ripristinano più giochi.
cli-invalid-backup-id = ID Backup invalido.
badge-failed = FALLITO
badge-duplicates = DUPLICATI
badge-duplicated = DUPLICATO
badge-ignored = IGNORATO
badge-redirected-from = DA: { $path }
badge-redirecting-to = A: { $path }
some-entries-failed = Alcune voci non sono riuscite a elaborare; cerca { badge-failed } nell'output per i dettagli. Controlla se è possibile accedere a questi file o se i loro percorsi sono molto lunghi.
cli-game-line-item-redirected = Reindirizzato da: { $path }
cli-game-line-item-redirecting = Reindirizzamento a: { $path }
button-backup = Backup
button-preview = Anteprima
button-restore = Ripristina
button-nav-backup = MODALITÀ BACKUP
button-nav-restore = MODALITÀ RIPRISTINO
button-nav-custom-games = GIOCHI PERSONALIZZATI
button-nav-other = ALTRO
button-add-root = Aggiungi radice
button-find-roots = Trova radici
button-add-redirect = Aggiungi reindirizzamento
button-add-game = Aggiungi gioco
button-continue = Continua
button-cancel = Annulla
button-cancelling = Annullamento...
button-okay = Va bene
button-select-all = Seleziona tutto
button-deselect-all = Deseleziona tutto
button-enable-all = Attiva tutto
button-disable-all = Disattiva tutto
button-customize = Personalizza
button-exit = Esci
no-roots-are-configured = Aggiungi alcune radici per eseguire il backup di ulteriori dati.
config-is-invalid = Errore: File di configurazione non valido.
manifest-is-invalid = Errore: File manifest non valido.
manifest-cannot-be-updated = Errore: Impossibile cercare aggiornamenti del file manifesto. La tua connessione Internet è inattiva?
cannot-prepare-backup-target = Errore: Impossibile preparare l'obiettivo di backup (sia creare che svuotare la cartella). Se hai la cartella aperta nel tuo file browser, prova a chiuderla: { $path }
restoration-source-is-invalid = Errore: la sorgente di ripristino non è valida (o non esiste o non è una directory). Si prega di ricontrollare la posizione: { $path }
registry-issue = Errore: Alcune voci del registro sono state saltate.
unable-to-browse-file-system = Errore: Impossibile navigare sul tuo sistema.
unable-to-open-directory = Errore: impossibile aprire la directory:
unable-to-open-url = Errore: Impossibile aprire l'URL:
processed-games =
    { $total-games } { $total-games ->
        [one] gioco
       *[other] giochi
    }
processed-games-subset =
    { $processed-games } di { $total-games } { $total-games ->
        [one] gioco
       *[other] giochi
    }
processed-size-subset = { $processed-size } di { $total-size }
field-backup-target = Backup su:
toggle-backup-merge = Unisci
field-restore-source = Ripristina da:
field-custom-files = Percorsi:
field-custom-registry = Registro:
field-search = Cerca:
field-sort = Ordina:
field-redirect-source =
    .placeholder = Origine (posizione originale)
field-redirect-target =
    .placeholder = Destinazione (nuova posizione)
field-backup-excluded-items = Esclusioni dal backup:
field-redirects = Reindirizzamenti:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Pieno:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differenziale:
field-backup-format = Formato:
field-backup-compression = Compressione:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Livello:
label-manifest = Manifesto
# This shows the time when we checked for an update to the manifest.
label-checked = Controllato
# This shows the time when we found an update to the manifest.
label-updated = Aggiornato
label-new = Nuovo
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Cartella principale
store-other-wine = Prefisso wine
store-other = Altro
sort-reversed = Invertita
backup-format-simple = Semplice
backup-format-zip = Comprimi in Zip
compression-none = Nessuna
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Chiaro
theme-dark = Scuro
redirect-bidirectional = Bidirezionale
explanation-for-exclude-store-screenshots =
    Nei backup, escludi screenshot specifici dello store. In questo momento, questo si applica solo
    agli screenshot { store-steam } che hai fatto. Se un gioco ha la propria funzionalità screenshot integrata, questa impostazione non influirà sul backup di questi
    screenshot.
consider-doing-a-preview =
    Se non lo hai già fatto, prendi in considerazione di fare un'anteprima prima in modo che non ci siano
    sorprese.
confirm-backup =
    Sei sicuro di voler procedere con il backup? { $path-action ->
        [merge] Nuovi dati di salvataggio verranno uniti nella cartella di destinazione:
        [recreate] La cartella di destinazione verrà eliminata e ricreata da zero:
       *[create] La cartella di destinazione verrà creata:
    }
confirm-restore =
    Sei sicuro di voler procedere con il ripristino?
    Questo sovrascriverà tutti i file attuali con i backup da qui:
confirm-add-missing-roots = Aggiungere queste radici?
no-missing-roots = Nessuna radice aggiuntiva trovata.
preparing-backup-target = Preparazione directory di backup...
updating-manifest = Aggiornamento manifest...
saves-found = Dati di salvataggio trovati.
no-saves-found = Nessun dato di salvataggio trovato.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = senza conferma

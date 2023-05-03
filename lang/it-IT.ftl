ludusavi = Ludusavi
language = Lingua
game-name = Nome
total-games = Giochi
file-size = Dimensione
file-location = Posizione
overall = Nel complesso
status = Status
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
button-comment = Comment
# This opens a download page.
button-get-app = Get { $app }
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
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
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
field-restore-source = Ripristina da:
field-custom-files = Percorsi:
field-custom-registry = Registro:
field-sort = Ordina:
field-redirect-source =
    .placeholder = Origine (posizione originale)
field-redirect-target =
    .placeholder = Destinazione (nuova posizione)
field-roots = Roots:
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
label-removed = Removed
label-comment = Comment
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
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
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
store-other-home = Cartella principale
store-other-wine = Prefisso wine
store-other = Altro
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
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots = Nei backup, escludi screenshot specifici dello store
consider-doing-a-preview =
    Se non lo hai già fatto, prendi in considerazione di fare un'anteprima prima in modo che non ci siano
    sorprese.
confirm-backup =
    Sei sicuro di voler procedere con il backup? { $path-action ->
        [merge] Nuovi dati di salvataggio verranno uniti nella cartella di destinazione:
       *[create] La cartella di destinazione verrà creata:
    }
confirm-restore =
    Sei sicuro di voler procedere con il ripristino?
    Questo sovrascriverà tutti i file attuali con i backup da qui:
confirm-cloud-upload =
    Do you want to synchronize your local files to the cloud?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to synchronize your cloud files to this system?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Aggiungere queste radici?
no-missing-roots = Nessuna radice aggiuntiva trovata.
loading = Loading...
preparing-backup-target = Preparazione directory di backup...
updating-manifest = Aggiornamento manifest...
no-cloud-changes = No changes to synchronize
saves-found = Dati di salvataggio trovati.
no-saves-found = Nessun dato di salvataggio trovato.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = senza conferma
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

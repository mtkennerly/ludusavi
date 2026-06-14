ludusavi = Ludusavi
language = Lingua
game-name = Nome
total-games = Giochi
file-size = Dimensione
file-location = Posizione
overall = Nel complesso
status = Stato
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
button-comment = Commento
button-lock = Blocca
button-unlock = Sblocca
# This opens a download page.
button-get-app = Ottieni { $app }
button-validate = Convalida
button-override-manifest = Sostituisci manifesto
button-extend-manifest = Estendi manifesto
button-sort = Ordina
button-download = Scarica
button-upload = Upload
button-ignore = Ignora
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
unable-to-configure-cloud = Impossibile configurare cloud.
unable-to-synchronize-with-cloud = Impossibile sincronizzare con il cloud.
cloud-synchronize-conflict = I backup locali e del cloud sono in conflitto. Apri Ludusavi ed esegui un caricamento o un download per risolvere il problema.
command-unlaunched = Comando non avviato: { $command }
command-terminated = Comando terminato improvvisamente: { $command }
command-failed = Comando fallito con codice { $code }: { $command }
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
field-roots = Radici:
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
label-removed = Rimosso
label-comment = Commento
label-unchanged = Invariato
label-backup = Backup
label-scan = Scansione
label-filter = Filtro
label-unique = Unico
label-complete = Completo
label-partial = Parziale
label-enabled = Abilitato
label-disabled = Disabilitato
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remote
label-remote-name = Nome remote
label-folder = Cartella
# An executable file
label-executable = Eseguibile
# Options given to a command line program
label-arguments = Argomenti
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Porta
label-username = Nome utente
label-password = Password
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Fornitore
label-custom = Personalizzato
label-none = Nessuno
label-change-count = Modifiche: { $total }
label-unscanned = Non scansionato
# This refers to a local file on the computer
label-file = File
label-game = Gioco
# Aliases are alternative titles for the same game.
label-alias = Pseudonimo
label-original-name = Nome originale
# Which manifest a game's data came from
label-source = Sorgente
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Manifesto primario
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integrazione
# This is a folder name where a specific game is installed
label-installed-name = Nome installato
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
store-other-home = Cartella principale
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Prefisso wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Disco Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Disco Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Disco Mac
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
reverse-redirects-when-restoring = Sequenza inversa dei reindirizzamenti durante il ripristino
show-disabled-games = Mostra giochi disattivati
show-unchanged-games = Mostra giochi invariati
show-unscanned-games = Mostra giochi non scansionati
override-max-threads = Sovrascrivi numero massimo thread
synchronize-automatically = Sincronizza automaticamente
prefer-alias-display = Mostra alias invece del nome originale
skip-unconstructive-backups = Salta il backup quando i dati saranno rimossi, ma non aggiunti o aggiornati
explanation-for-exclude-store-screenshots = Nei backup, escludi screenshot specifici dello store
explanation-for-exclude-cloud-games = Non eseguire il backup dei giochi con supporto cloud su queste piattaforme
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
    Vuoi sostituire i tuoi file cloud con quelli locali?
    I tuoi file cloud ({ $cloud-path }) diventeranno una copia esatta dei tuoi file locali ({ $local-path }).
    I file nel cloud verranno aggiornati o cancellati se necessario.
confirm-cloud-download =
    Vuoi sostituire i tuoi file locali con quelli cloud?
    I tuoi file locali ({ $local-path }) diventeranno una copia esatta dei file cloud ({ $cloud-path }).
    I file locali saranno aggiornati o cancellati se necessario.
confirm-add-missing-roots = Aggiungere queste radici?
no-missing-roots = Nessuna radice aggiuntiva trovata.
loading = Caricamento in corso...
preparing-backup-target = Preparazione directory di backup...
updating-manifest = Aggiornamento manifest...
no-cloud-changes = Nessuna modifica da sincronizzare
backups-are-valid = I tuoi backup sono validi.
backups-are-invalid =
    I backup di questi giochi sembrano non essere validi.
    Vuoi creare nuovi backup completi per questi giochi?
saves-found = Dati di salvataggio trovati.
no-saves-found = Nessun dato di salvataggio trovato.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = senza conferma
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = riavvio richiesto
prefix-error = Errore: { $message }
prefix-warning = Attenzione: { $message }
cloud-app-unavailable = I backup cloud sono disabilitati perché { $app } non è disponibile.
cloud-not-configured = I backup cloud sono disabilitati perché non è stato configurato alcun sistema cloud.
cloud-path-invalid = I backup cloud sono disabilitati perché il percorso di backup non è valido.
game-is-unrecognized = Ludusavi non riconosce questo gioco.
game-has-nothing-to-restore = Questo gioco non dispone di un backup da ripristinare.
launch-game-after-error = Avviare comunque il gioco?
game-did-not-launch = Lancio del gioco fallito.
backup-is-newer-than-current-data = Il backup esistente è più recente dei dati attuali.
backup-is-older-than-current-data = Il backup esistente è più vecchio dei dati attuali.
back-up-specific-game =
    .confirm = Backup dei dati di salvataggio per { $game }?
    .failed = Backup dei dati di salvataggio di { $game } non riuscito
restore-specific-game =
    .confirm = Ripristina i dati di salvataggio per { $game }?
    .failed = Ripristino dei dati di salvataggio per { $game } non riuscito
new-version-check = Controlla automaticamente gli aggiornamenti dell'applicazione
new-version-available = Un aggiornamento dell'applicazione è disponibile: { $version }. Vuoi visualizzare le note di rilascio?
custom-game-will-override = Questo gioco personalizzato sovrascrive una voce del manifesto
custom-game-will-extend = Questo gioco personalizzato estende una voce del manifesto
operation-will-only-include-listed-games = Questo processerà solo i giochi che sono attualmente elencati

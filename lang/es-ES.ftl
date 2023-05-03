ludusavi = Ludusavi
language = Idioma
game-name = Nombre
total-games = Juegos
file-size = Tamaño
file-location = Ubicación
overall = Global
status = Estatus
cli-unrecognized-games = No hay información para estos juegos:
cli-unable-to-request-confirmation = No se pudo solicitar confirmación.
    .winpty-workaround = Si estás usando un emulador de Bash (como Git Bash), intenta ejecutar winpty.
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.
badge-failed = FALLADO
badge-duplicates = DUPLICADOS
badge-duplicated = DUPLICADO
badge-ignored = IGNORADO
badge-redirected-from = DESDE: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = Algunas entradas no se han podido procesar; busca { badge-failed } en la salida para ver los detalles. Comprueba si puedes acceder a esos archivos o si sus rutas son muy largas.
cli-game-line-item-redirected = Redirigido de: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
button-backup = Respaldar
button-preview = Previsualizar
button-restore = Restaurar
button-nav-backup = MODO DE RESPALDO
button-nav-restore = MODO DE RESTAURACIÓN
button-nav-custom-games = JUEGOS PERSONALIZADOS
button-nav-other = OTROS
button-add-game = Añadir juego
button-continue = Continuar
button-cancel = Cancelar
button-cancelling = Cancelando...
button-okay = De acuerdo
button-select-all = Seleccionar todos
button-deselect-all = Deseleccionar todos
button-enable-all = Habilitar todos
button-disable-all = Deshabilitar todos
button-customize = Customize
button-exit = Salir
button-comment = Comment
# This opens a download page.
button-get-app = Get { $app }
no-roots-are-configured = Añade algunas raíces para respaldar aún más datos.
config-is-invalid = Error: El archivo de configuración no es válido.
manifest-is-invalid = Error: El archivo de manifiesto no es válido.
manifest-cannot-be-updated = Error: No se ha podido comprobar la actualización del archivo de manifiesto. ¿Se ha caído la conexión a Internet?
cannot-prepare-backup-target = Error: No se pudo preparar el destino de la copia de seguridad (creando o vaciando la carpeta). Si tiene la carpeta abierta en su navegador de archivos, intente cerrarla: { $path }
restoration-source-is-invalid = Error: La fuente de restauración no es válida (no existe o no es un directorio). Por favor, comprueba la ubicación: { $path }
registry-issue = Error: Se omitieron algunas entradas del registro.
unable-to-browse-file-system = Error: No se puede navegar en su sistema.
unable-to-open-directory = Error: no se puede abrir el directorio:
unable-to-open-url = Error: No se puede abrir la URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] juego
       *[other] juegos
    }
processed-games-subset =
    { $processed-games } de { $total-games } { $total-games ->
        [one] juego
       *[other] juegos
    }
processed-size-subset = { $processed-size } de { $total-size }
field-backup-target = Respaldar a:
field-restore-source = Restaurar desde:
field-custom-files = Rutas:
field-custom-registry = Registro:
field-sort = Ordenar por:
field-redirect-source =
    .placeholder = Origen (ubicación original)
field-redirect-target =
    .placeholder = Destino (nueva ubicación)
field-roots = Roots:
field-backup-excluded-items = Backup exclusions:
field-redirects = Redirects:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Level:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Checked
# This shows the time when we found an update to the manifest.
label-updated = Updated
label-new = New
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
store-other-home = Carpeta Home
store-other-wine = Prefijo de Wine
store-other = Otro
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = None
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Blanco
theme-dark = Oscuro
redirect-bidirectional = Bidirectional
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots = En las copias de seguridad, excluye las capturas de pantalla específicas de la tienda
consider-doing-a-preview =
    Si aún no lo has hecho, considera hacer una vista previa primero para que
    no haya sorpresas.
confirm-backup =
    ¿Estás seguro de que quieres proceder con la copia de seguridad? { $path-action ->
        [merge] Los nuevos datos guardados se combinaran en la carpeta de destino:
       *[create] Se creará la carpeta de destino:
    }
confirm-restore =
    ¿Estás seguro de que deseas continuar con la restauración?
    Esto sobrescribirá cualquier archivo actual con las copias de seguridad desde aquí:
confirm-cloud-upload =
    Do you want to synchronize your local files to the cloud?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to synchronize your cloud files to this system?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
loading = Loading...
preparing-backup-target = Preparing backup directory...
updating-manifest = Updating manifest...
no-cloud-changes = No changes to synchronize
saves-found = Save data found.
no-saves-found = No save data found.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = no confirmation
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

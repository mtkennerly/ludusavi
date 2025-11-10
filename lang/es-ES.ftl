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
cli-backup-id-with-multiple-games = No se puede especificar el ID de copia de seguridad al restaurar múltiples juegos.
cli-invalid-backup-id = ID de copia de seguridad inválido.
badge-failed = FALLADO
badge-duplicates = DUPLICADOS
badge-duplicated = DUPLICADO
badge-ignored = IGNORADO
badge-redirected-from = DESDE: { $path }
badge-redirecting-to = A: { $path }
some-entries-failed = Algunas entradas no se han podido procesar; busca { badge-failed } en la salida para ver los detalles. Comprueba si puedes acceder a esos archivos o si sus rutas son muy largas.
cli-game-line-item-redirected = Redirigido de: { $path }
cli-game-line-item-redirecting = Redirigiendo a: { $path }
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
button-customize = Personalizar
button-exit = Salir
button-comment = Comentar
button-lock = Bloquear
button-unlock = Desbloquear
# This opens a download page.
button-get-app = Obtener { $app }
button-validate = Validar
button-override-manifest = Reemplazar manifiesto
button-extend-manifest = Extender manifiesto
button-sort = Ordenar
button-download = Descargar
button-upload = Subir
button-ignore = Ignorar
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
unable-to-configure-cloud = No se ha podido configurar la nube.
unable-to-synchronize-with-cloud = No se ha podido sincronizar con la nube.
cloud-synchronize-conflict = Tus copias de seguridad locales y en la nube están en conflicto. Realiza una subida o descarga para resolver esto.
command-unlaunched = El comando no se inició: { $command }
command-terminated = Comando finalizado abruptamente: { $command }
command-failed = Comando falló con el código { $code }: { $command }
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
field-roots = Raíces:
field-backup-excluded-items = Exclusiones de copia de seguridad:
field-redirects = Redirecciones:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Completo:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Diferencial:
field-backup-format = Formato:
field-backup-compression = Compresión:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Nivel:
label-manifest = Manifiesto
# This shows the time when we checked for an update to the manifest.
label-checked = Marcado
# This shows the time when we found an update to the manifest.
label-updated = Actualizado
label-new = Nuevo
label-removed = Eliminado
label-comment = Comentario
label-unchanged = Sin cambios
label-backup = Copia de seguridad
label-scan = Escanear
label-filter = Filtro
label-unique = Único
label-complete = Completado
label-partial = Parcial
label-enabled = Habilitado
label-disabled = Deshabilitado
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Hilos
label-cloud = Nube
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remoto
label-remote-name = Nombre remoto
label-folder = Carpeta
# An executable file
label-executable = Ejecutable
# Options given to a command line program
label-arguments = Argumentos
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Anfitrión
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Puerto
label-username = Nombre de usuario
label-password = Contraseña
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Proveedor
label-custom = Personalizado
label-none = Ninguno
label-change-count = Cambios: { $total }
label-unscanned = Sin escanear
# This refers to a local file on the computer
label-file = Archivo
label-game = Juego
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Nombre original
# Which manifest a game's data came from
label-source = Fuente
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Manifiesto primario
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integración
# This is a folder name where a specific game is installed
label-installed-name = Nombre de instalación
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
store-other-home = Carpeta Home
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Prefijo de Wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Unidad de Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Unidad de Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Unidad de Mac
store-other = Otro
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = Ninguno
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Claro
theme-dark = Oscuro
redirect-bidirectional = Bidireccional
reverse-redirects-when-restoring = Invertir secuencia de redirecciones al restaurar
show-disabled-games = Mostrar juegos desactivados
show-unchanged-games = Mostrar juegos sin cambios
show-unscanned-games = Mostrar juegos no escaneados
override-max-threads = Anular hilos máximos
synchronize-automatically = Sincronizar automáticamente
prefer-alias-display = Mostrar alias en lugar del nombre original
skip-unconstructive-backups = Saltar la copia de seguridad cuando solo se van a eliminar datos, pero no se va a agregar ni actualizar nada
explanation-for-exclude-store-screenshots = En las copias de seguridad, excluye las capturas de pantalla específicas de la tienda
explanation-for-exclude-cloud-games = No hacer copias de seguridad de juegos con soporte en la nube en estas plataformas
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
    ¿Quieres reemplazar tus archivos en la nube con tus archivos locales?
    Los archivos en la nube ({ $cloud-path }) se convertirán en una copia exacta de tus archivos locales ({ $local-path }).
    Los archivos en la nube serán actualizados o eliminados según sea necesario.
confirm-cloud-download =
    ¿Quieres reemplazar tus archivos locales por tus archivos en la nube?
    Tus archivos locales ({ $local-path }) se convertirán en una copia exacta de tus archivos en la nube ({ $cloud-path }).
    Los archivos locales serán actualizados o eliminados según sea necesario.
confirm-add-missing-roots = ¿Añadir estas raíces?
no-missing-roots = No se han encontrado raíces adicionales.
loading = Cargando...
preparing-backup-target = Preparando directorio de copia de seguridad...
updating-manifest = Actualizando manifiesto...
no-cloud-changes = No hay cambios para sincronizar
backups-are-valid = Tus copias de seguridad son válidas.
backups-are-invalid =
    Las copias de seguridad de estos juegos parecen ser inválidas.
    ¿Quieres crear nuevas copias de seguridad completas para estos juegos?
saves-found = Datos de guardado encontrados.
no-saves-found = Datos de guardado no encontrados.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = sin confirmación
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = reinicio requerido
prefix-error = Error: { $message }
prefix-warning = Advertencia: { $message }
cloud-app-unavailable = Las copias de seguridad de la nube están deshabilitadas porque { $app } no está disponible.
cloud-not-configured = Las copias de seguridad de la nube están desactivadas porque no se ha configurado ningún sistema de nube.
cloud-path-invalid = Las copias de seguridad de la nube están desactivadas porque la ruta de la copia de seguridad no es válida.
game-is-unrecognized = Ludusavi no reconoce este juego.
game-has-nothing-to-restore = Este juego no tiene una copia de seguridad para restaurar.
launch-game-after-error = ¿Iniciar el juego de todos modos?
game-did-not-launch = El juego no se pudo iniciar.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = La copia de seguridad existente es más antigua que los datos actuales.
back-up-specific-game =
    .confirm = ¿Respaldar datos guardados de { $game }?
    .failed = Error al realizar la copia de seguridad de los datos guardados de { $game }
restore-specific-game =
    .confirm = ¿Restaurar datos guardados de { $game }?
    .failed = Error al restaurar los datos guardados de { $game }
new-version-check = Comprobar actualizaciones automáticamente
new-version-available = Una actualización de la aplicación está disponible: { $version }. ¿Desea ver las notas del lanzamiento?
custom-game-will-override = Este juego personalizado reemplaza una entrada de manifiesto
custom-game-will-extend = Este juego personalizado extiende una entrada de manifiesto
operation-will-only-include-listed-games = Esto solo procesará los juegos que se encuentran actualmente listados

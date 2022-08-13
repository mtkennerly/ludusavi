ludusavi = Ludusavi
language = Idioma
language-font-compatibility = Some languages may require a custom font.
font = Fuente
cli-backup-target-already-exists = El objetivo de la copia de seguridad ya existe ( { $path } ). Elija un --path diferente o borre con --force.
cli-unrecognized-games = No hay información para estos juegos:
cli-confirm-restoration = ¿Quieres restaurar desde { $path }?
cli-unable-to-request-confirmation = No se pudo solicitar confirmación.
    .winpty-workaround = Si estás usando un emulador de Bash (como Git Bash), intenta ejecutar winpty.
badge-failed = FALLADO
badge-duplicates = DUPLICADOS
badge-duplicated = DUPLICADO
badge-ignored = IGNORADO
badge-redirected-from = DESDE: { $path }
some-entries-failed = Algunas entradas no se han podido procesar; busca { label-failed } en la salida para ver los detalles. Comprueba si puedes acceder a esos archivos o si sus rutas son muy largas.
cli-game-line-item-redirected = Redirigido de: { $path }
cli-summary =
    .succeeded =
        Global:
          Juegos: { $total-games }
          Tamaño: { $total-size }
          Ubicación: { $path }
    .failed =
        Global:
          Juegos: { $processed-games } de { $total-games }
          Tamaño: { $processed-size } de { $total-size }
          Ubicación: { $path }
button-backup = Respaldar
button-preview = Previsualizar
button-restore = Restaurar
button-nav-backup = MODO DE RESPALDO
button-nav-restore = MODO DE RESTAURACIÓN
button-nav-custom-games = JUEGOS PERSONALIZADOS
button-nav-other = OTROS
button-add-root = Añadir raíz
button-find-roots = Find roots
button-add-redirect = Añadir redirección
button-add-game = Añadir juego
button-continue = Continuar
button-cancel = Cancelar
button-cancelling = Cancelando...
button-okay = De acuerdo
button-select-all = Seleccionar todos
button-deselect-all = Deseleccionar todos
button-enable-all = Habilitar todos
button-disable-all = Deshabilitar todos
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
toggle-backup-merge = Combinar
field-restore-source = Restaurar desde:
field-custom-files = Rutas:
field-custom-registry = Registro:
field-search = Buscar:
field-sort = Sort:
field-redirect-source =
    .placeholder = Origen (ubicación original)
field-redirect-target =
    .placeholder = Destino (nueva ubicación)
field-custom-game-name =
    .placeholder = Nombre
field-search-game-name =
    .placeholder = Nombre
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
store-other-home = Carpeta Home
store-other-wine = Prefijo de Wine
store-other = Otro
sort-name = Name
sort-size = Size
sort-reversed = Reversed
explanation-for-exclude-other-os-data =
    En las copias de seguridad, excluye las ubicaciones de guardado que sólo han sido confirmadas en otro
    sistema operativo. Algunos juegos siempre colocan las partidas guardadas en el mismo lugar, pero las
    pero las ubicaciones pueden haber sido confirmadas sólo para un sistema operativo diferente, por lo que puede ayudar comprobarlas de todos modos. Excluir esos datos puede ayudar a evitar falsos positivos,
    pero también puede significar perder algunos datos de guardado. En Linux, las copias de seguridad de Proton se guardan independientemente de esta configuración.
explanation-for-exclude-store-screenshots =
    En las copias de seguridad, excluye las capturas de pantalla específicas de la tienda. En este momento, esto sólo se aplica a las capturas de pantalla { store-steam } que has tomado. Si un juego tiene su propia funcionalidad de funcionalidad de capturas de pantalla, este ajuste no afectará a si esas
    capturas de pantalla son respaldadas.
consider-doing-a-preview =
    Si aún no lo has hecho, considera hacer una vista previa primero para que
    no haya sorpresas.
confirm-backup =
    ¿Estás seguro de que quieres proceder con la copia de seguridad? { $path-action ->
        [merge] Los nuevos datos guardados se combinaran en la carpeta de destino
        [recreate] La carpeta de destino será eliminada y recreada desde cero
       *[create] Se creará la carpeta de destino
    }:

    { $path }

    { consider-doing-a-preview }
confirm-restore =
    ¿Estás seguro de que deseas continuar con la restauración?
    Esto sobrescribirá cualquier archivo actual con las copias de seguridad desde aquí:

    { $path }

    { consider-doing-a-preview }
confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
preparing-backup-target = Preparing backup directory...

ludusavi = Ludusavi
language = Idioma
game-name = Nome
total-games = Jogos
file-size = Tamanho
file-location = Localização
overall = Geral
status = Status
cli-unrecognized-games = Sem informações para estes jogos:
cli-unable-to-request-confirmation = Não foi possível solicitar confirmação.
    .winpty-workaround = Se você estiver usando um emulador Bash (como Git Bash), tente executar a winpty.
cli-backup-id-with-multiple-games = Não é possível especificar a ID do backup ao restaurar vários jogos.
cli-invalid-backup-id = ID do backup inválido.
badge-failed = FALHOU
badge-duplicates = DUPLICADOS
badge-duplicated = DUPLICADO
badge-ignored = IGNORADO
badge-redirected-from = DE: { $path }
badge-redirecting-to = PARA: { $path }
some-entries-failed = Algumas entradas não conseguiram processar; procure por { badge-failed } na saída para mais detalhes. Verifique se você pode acessar esses arquivos ou se os caminhos deles são muito longos.
cli-game-line-item-redirected = Redirecionado de: { $path }
cli-game-line-item-redirecting = Redirecionando para: { $path }
button-backup = Fazer backup
button-preview = Visualizar
button-restore = Restaurar
button-nav-backup = MODO DE BACKUP
button-nav-restore = MODO DE RESTAURAÇÃO
button-nav-custom-games = JOGOS PERSONALIZADOS
button-nav-other = OUTRO
button-add-game = Adicionar jogo
button-continue = Continuar
button-cancel = Cancelar
button-cancelling = Cancelamento...
button-okay = Ok
button-select-all = Selecionar tudo
button-deselect-all = Desmarcar tudo
button-enable-all = Ativar tudo
button-disable-all = Desativar tudo
button-customize = Personalizar
button-exit = Sair
button-comment = Comentário
# This opens a download page.
button-get-app = Get { $app }
no-roots-are-configured = Adicione algumas raízes para armazenar ainda mais dados.
config-is-invalid = Erro: O arquivo de configuração é inválido.
manifest-is-invalid = Erro: O arquivo de manifesto é inválido.
manifest-cannot-be-updated = Erro: Não foi possível verificar se há uma atualização no manifesto. Sua conexão com a Internet está inativa?
cannot-prepare-backup-target = Erro: Não é possível preparar o destino do backup (criando ou esvaziando a pasta). Se você tiver a pasta aberta no seu navegador de arquivos, tente fechá-la: { $path }
restoration-source-is-invalid = Erro: A fonte de restauração é inválida (ou não existe ou não é um diretório). Por favor, verifique o local: { $path }
registry-issue = Erro: Algumas entradas de registro foram ignoradas.
unable-to-browse-file-system = Erro: Não é possível navegar no seu sistema.
unable-to-open-directory = Erro: Não é possível abrir o diretório:
unable-to-open-url = Erro: Não foi possível abrir a URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] jogo
       *[other] jogos
    }
processed-games-subset =
    { $processed-games } de { $total-games } { $total-games ->
        [one] jogo
       *[other] jogos
    }
processed-size-subset = { $processed-size } de { $total-size }
field-backup-target = Fazer backup para:
field-restore-source = Restaurar de:
field-custom-files = Caminhos:
field-custom-registry = Registro:
field-sort = Organizar:
field-redirect-source =
    .placeholder = Fonte (local original)
field-redirect-target =
    .placeholder = Alvo (novo local)
field-roots = Raiz:
field-backup-excluded-items = Exclusões do backup:
field-redirects = Redirecionar:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Todos:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Diferencial:
field-backup-format = Formato:
field-backup-compression = Compressão:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Nível:
label-manifest = Manifesto
# This shows the time when we checked for an update to the manifest.
label-checked = Verificado
# This shows the time when we found an update to the manifest.
label-updated = Atualizado
label-new = Novo
label-removed = Removed
label-comment = Comentário
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
store-other-home = Pasta padrão
store-other-wine = Prefixo Wine
store-other = Outro
backup-format-simple = Simples
backup-format-zip = Zip
compression-none = Nenhum
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Claro
theme-dark = Escuro
redirect-bidirectional = Bidirecional
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots =
    Nos backups, exclui capturas de tela específicas de armazenamento
consider-doing-a-preview =
    Se você ainda não fez, considere fazer uma pré-visualização primeiro, então
    não há surpresas.
confirm-backup =
    Tem certeza que deseja prosseguir com o backup? { $path-action ->
        [merge] Novos dados salvos serão mesclados na pasta de destino:
       *[create] A pasta de destino será criada:
    }
confirm-restore =
    Tem certeza que deseja prosseguir com a restauração?
    Isto irá sobrescrever qualquer arquivo atual com os backups aqui:
confirm-cloud-upload =
    Do you want to synchronize your local files to the cloud?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to synchronize your cloud files to this system?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Adicionar estas origens?
no-missing-roots = Nenhuma origem adicional encontrada.
loading = Loading...
preparing-backup-target = Preparando o diretório de backup...
updating-manifest = Atualizando manifesto...
no-cloud-changes = No changes to synchronize
saves-found = Dados salvos encontrados.
no-saves-found = Não foram encontrados dados salvos.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = sem confirmação
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

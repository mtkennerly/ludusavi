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
button-lock = Travar
button-unlock = Destravar
# This opens a download page.
button-get-app = Obter { $app }
button-validate = Validar
button-override-manifest = Sobrescrever manifesto
button-extend-manifest = Estender manifesto
button-sort = Classificar
button-download = Download
button-upload = Upload
button-ignore = Ignorar
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
unable-to-configure-cloud = Não foi possível configurar a nuvem.
unable-to-synchronize-with-cloud = Não foi possível sincronizar com a nuvem.
cloud-synchronize-conflict = Seus backups locais e da nuvem estão em conflito. Execute um upload ou download para resolver isso.
command-unlaunched = Comando não iniciou: { $command }
command-terminated = Comando encerrado abruptamente: { $command }
command-failed = O comando falhou com o código { $code }: { $command }
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
label-removed = Removido
label-comment = Comentário
label-unchanged = Inalterada
label-backup = Backup
label-scan = Escanear
label-filter = Filtro
label-unique = Único
label-complete = Concluído
label-partial = Parcial
label-enabled = Ativado
label-disabled = Desativado
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Tópicos
label-cloud = Nuvem
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remoto
label-remote-name = Nome remoto
label-folder = Pasta
# An executable file
label-executable = Executável
# Options given to a command line program
label-arguments = Argumentos
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Hospedeiro
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Porta
label-username = Nome de usuário
label-password = Senha
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Provedor
label-custom = Personalizado
label-none = Nenhum
label-change-count = Mudanças: { $total }
label-unscanned = Não verificado
# This refers to a local file on the computer
label-file = Arquivo
label-game = Jogo
# Aliases are alternative titles for the same game.
label-alias = Apelido
label-original-name = Nome original
# Which manifest a game's data came from
label-source = Fonte
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Manifesto primário
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integração
# This is a folder name where a specific game is installed
label-installed-name = Nome Instalado
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
store-other-home = Pasta padrão
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Prefixo Wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Drive do Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Drive do Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Drive do Mac
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
reverse-redirects-when-restoring = Reverter sequência de redirecionamentos durante restauração
show-disabled-games = Mostrar jogos desativados
show-unchanged-games = Mostrar jogos inalterados
show-unscanned-games = Mostrar jogos não escaneados
override-max-threads = Substituir o número máximo de threads
synchronize-automatically = Sincronizar automaticamente
prefer-alias-display = Exibir apelido ao invés do nome original
skip-unconstructive-backups = Pular backup quando dados serão removidos ao invés de adicionados ou atualizados
explanation-for-exclude-store-screenshots = Nos backups, exclui capturas de tela específicas de armazenamento
explanation-for-exclude-cloud-games = Não faça backup de jogos com suporte à nuvem nessas plataformas
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
    Você quer substituir seus arquivos na nuvem por seus arquivos locais?
    Seus arquivos da nuvem ({ $cloud-path }) se tornarão uma cópia exata de seus arquivos locais ({ $local-path }).
    Arquivos na nuvem serão atualizados ou excluídos conforme necessário.
confirm-cloud-download =
    Deseja substituir seus arquivos locais por seus arquivos na nuvem?
    Seus arquivos locais ({ $local-path }) se tornará uma cópia exata dos seus arquivos de nuvem ({ $cloud-path }).
    Os arquivos locais serão atualizados ou excluídos conforme necessário.
confirm-add-missing-roots = Adicionar estas origens?
no-missing-roots = Nenhuma origem adicional encontrada.
loading = Carregando...
preparing-backup-target = Preparando o diretório de backup...
updating-manifest = Atualizando manifesto...
no-cloud-changes = Não há alterações para sincronizar
backups-are-valid = Seus backups são válidos.
backups-are-invalid =
    Os backups destes jogos parecem inválidos.
    Você deseja criar novos backups completos para esses jogos?
saves-found = Dados salvos encontrados.
no-saves-found = Não foram encontrados dados salvos.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = sem confirmação
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = é necessário reiniciar
prefix-error = Erro: { $message }
prefix-warning = Aviso: { $message }
cloud-app-unavailable = Backups na nuvem estão desativados porque { $app } não está disponível.
cloud-not-configured = Backups na nuvem estão desativados porque nenhum sistema na nuvem está configurado.
cloud-path-invalid = Backups na nuvem estão desativados porque o caminho de backup é inválido.
game-is-unrecognized = Este jogo não foi reconhecido pelo Ludusavi.
game-has-nothing-to-restore = Este jogo não tem um backup para restauração.
launch-game-after-error = Iniciar o jogo de qualquer forma?
game-did-not-launch = Jogo falhou ao iniciar.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = O backup existe é mais antigo que os dados atuais.
back-up-specific-game =
    .confirm = Fazer backup dos dados de { $game }?
    .failed = Falha ao fazer backup dos dados de { $game }
restore-specific-game =
    .confirm = Restaurar dados salvos de { $game }?
    .failed = Falha ao restaurar dados de { $game }
new-version-check = Verificar por atualizações do aplicativo automaticamente
new-version-available = Uma atualização do aplicativo está disponível: { $version }. Gostaria de ver as notas de lançamento?
custom-game-will-override = Esse jogo personalizado substitui uma entrada de manifesto
custom-game-will-extend = Este jogo personalizado estende uma entrada de manifesto
operation-will-only-include-listed-games = Isso processará apenas os jogos que estão listados no momento

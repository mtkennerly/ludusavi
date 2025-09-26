ludusavi = Ludusavi
language = Мова
game-name = Назва гри
total-games = Всі ігри
file-size = Розмір файлів
file-location = Розташування
overall = Overall
status = Статус
cli-unrecognized-games = Немає інформації про ці ігри:
cli-unable-to-request-confirmation = Неможливо запитати підтвердження.
    .winpty-workaround = Якщо ви використовуєте емулятор Bash (наприклад, Git Bash), спробуйте запустити winpty.
cli-backup-id-with-multiple-games = Неможливо вказати ідентифікатор резервної копії під час відновлення кількох ігор.
cli-invalid-backup-id = Недійсний ідентифікатор резервної копії.
badge-failed = Помилка
badge-duplicates = ДУБЛІКАТИ
badge-duplicated = ПРОДУБЛЮВАНО
badge-ignored = ІГНОРУЄТЬСЯ
badge-redirected-from = ВІД: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = Some entries failed to process; look for { badge-failed } in the output for details. Double check whether you can access those files or whether their paths are very long.
cli-game-line-item-redirected = Redirected from: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
button-backup = Резервне копіювання
button-preview = Попередній перегляд
button-restore = Відновлення
button-nav-backup = РЕЗЕРВНИЙ РЕЖИМ
button-nav-restore = РЕЖИМ ВІДНОВЛЕННЯ
button-nav-custom-games = CUSTOM GAMES
button-nav-other = ІНШЕ
button-add-game = Додати гру
button-continue = Продовжити
button-cancel = Скасувати
button-cancelling = Скасування...
button-okay = Гаразд
button-select-all = Вибрати все
button-deselect-all = Deselect all
button-enable-all = Увімкнути всі
button-disable-all = Відключити все
button-customize = Налаштувати
button-exit = Вихід
button-comment = Коментар
button-lock = Lock
button-unlock = Unlock
# This opens a download page.
button-get-app = Отримати {$app}
button-validate = Перевірити
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sort
button-download = Download
button-upload = Upload
button-ignore = Ignore
no-roots-are-configured = Додайте кілька коренів, щоб створити резервну копію ще більше даних.
config-is-invalid = Помилка: файл конфігурації недійсний.
manifest-is-invalid = Помилка: файл маніфесту недійсний.
manifest-cannot-be-updated = Помилка: неможливо перевірити наявність оновлення файлу маніфесту. Ваше інтернет-з’єднання не працює?
cannot-prepare-backup-target = Помилка: неможливо підготувати ціль резервного копіювання (створення або очищення папки). Якщо папка відкрита в браузері файлів, спробуйте закрити її: { $path }
restoration-source-is-invalid = Помилка: джерело відновлення недійсне (або не існує, або не є каталогом). Ще раз перевірте розташування: { $path }
registry-issue = Помилка: деякі записи реєстру було пропущено.
unable-to-browse-file-system = Помилка: неможливо переглянути у вашій системі.
unable-to-open-directory = Помилка: неможливо відкрити каталог:
unable-to-open-url = Помилка: Не вдається відкрити URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Команда не запущена: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Помилка команди з кодом { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] game
       *[other] games
    }
processed-games-subset =
    { $processed-games } of { $total-games } { $total-games ->
        [one] game
       *[other] games
    }
processed-size-subset = { $processed-size } of { $total-size }
field-backup-target = Резервне копіювання до:
field-restore-source = Відновити з:
field-custom-files = Шляхи:
field-custom-registry = Реєстр:
field-sort = Sort:
field-redirect-source =
    .placeholder = Джерело (початкове розташування)
field-redirect-target =
    .placeholder = Ціль (нове місце)
field-roots = Коріння:
field-backup-excluded-items = Резервні виключення:
field-redirects = Перенаправлення:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Повний:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Формат:
field-backup-compression = Стиснення:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Рівень:
label-manifest = Маніфест
# This shows the time when we checked for an update to the manifest.
label-checked = Перевірено
# This shows the time when we found an update to the manifest.
label-updated = Оновлено
label-new = Нове
label-removed = Видалено
label-comment = Коментар
label-unchanged = Без змін
label-backup = Backup
label-scan = Сканувати
label-filter = Фільтр
label-unique = Унікальний
label-complete = Завершено
label-partial = Частковий
label-enabled = Увімкнено
label-disabled = Вимкнено
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Хмара
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Дистанційно
label-remote-name = Віддалена назва
label-folder = Папка
# An executable file
label-executable = Executable
# Options given to a command line program
label-arguments = Аргументи
label-url = URL-адреса
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Хост
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Порт
label-username = Ім'я користувача
label-password = Пароль
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Провайдер
label-custom = Custom
label-none = Жодного
label-change-count = Зміни: { $total }
label-unscanned = Не просканований
# This refers to a local file on the computer
label-file = Файл
label-game = Гра
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Original name
# Which manifest a game's data came from
label-source = Source
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primary manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integration
# This is a folder name where a specific game is installed
label-installed-name = Installed name
store-ea = ЕА
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
store-other-home = Домашня папка
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine prefix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Диск Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Диск Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac диск
store-other = Інше
backup-format-simple = Просто
backup-format-zip = Zip
compression-none = Жодного
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Тема
theme-light = Світло
theme-dark = Темно
redirect-bidirectional = Двонаправлений
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = Показати незмінні ігри
show-unscanned-games = Показати нескановані ігри
override-max-threads = Перевизначити максимальну кількість потоків
synchronize-automatically = Автоматична синхронізація
prefer-alias-display = Display alias instead of original name
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = In backups, exclude store-specific screenshots
explanation-for-exclude-cloud-games = Do not back up games with cloud support on these platforms
consider-doing-a-preview =
    If you haven't already, consider doing a preview first so that there
    are no surprises.
confirm-backup =
    Are you sure you want to proceed with the backup? { $path-action ->
        [merge] New save data will be merged into the target folder:
       *[create] The target folder will be created:
    }
confirm-restore =
    Are you sure you want to proceed with the restoration?
    This will overwrite any current files with the backups from here:
confirm-cloud-upload =
    Do you want to replace your cloud files with your local files?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to replace your local files with your cloud files?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
loading = Loading...
preparing-backup-target = Preparing backup directory...
updating-manifest = Updating manifest...
no-cloud-changes = No changes to synchronize
backups-are-valid = Your backups are valid.
backups-are-invalid =
    These games' backups appear to be invalid.
    Do you want to create new full backups for these games?
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
game-is-unrecognized = Ludusavi does not recognize this game.
game-has-nothing-to-restore = This game does not have a backup to restore.
launch-game-after-error = Launch the game anyway?
game-did-not-launch = Game failed to launch.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Back up save data for { $game }?
    .failed = Failed to back up save data for { $game }
restore-specific-game =
    .confirm = Restore save data for { $game }?
    .failed = Failed to restore save data for { $game }
new-version-check = Check for application updates automatically
new-version-available = An application update is available: { $version }. Would you like to view the release notes?
custom-game-will-override = This custom game overrides a manifest entry
custom-game-will-extend = This custom game extends a manifest entry
operation-will-only-include-listed-games = This will only process the games that are currently listed

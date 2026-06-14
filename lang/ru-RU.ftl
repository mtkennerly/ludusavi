ludusavi = Ludusavi
language = Язык
game-name = Название
total-games = Игры
file-size = Размер
file-location = Местоположение
overall = Всего
status = Статус
cli-unrecognized-games = Нет информации об этих играх:
cli-unable-to-request-confirmation = Не удалось запросить подтверждение.
    .winpty-workaround = Если вы используете эмулятор Bash (например Git Bash), попробуйте запустить winpty.
cli-backup-id-with-multiple-games = Невозможно задать идентификатор резервной копии при восстановлении нескольких игр.
cli-invalid-backup-id = Неверный идентификатор резервной копии.
badge-failed = ОШИБКА
badge-duplicates = ДУБЛИКАТ
badge-duplicated = ДУБЛИРОВАННЫЙ
badge-ignored = ИГНОРИРОВАН
badge-redirected-from = ИЗ: { $path }
badge-redirecting-to = В: { $path }
some-entries-failed = Некоторые записи не удалось обработать; ищите { badge-failed } на выходе для получения деталей. Дважды проверьте, имеют ли вы доступ к этим файлам или их пути очень длинные.
cli-game-line-item-redirected = Перенаправлено из: { $path }
cli-game-line-item-redirecting = Перенаправление в: { $path }
button-backup = Резервирование
button-preview = Предпросмотр
button-restore = Восстановить
button-nav-backup = РЕЗЕРВИРОВАНИЕ
button-nav-restore = ВОССТАНОВЛЕНИЕ
button-nav-custom-games = СВОЯ ИГРА
button-nav-other = ДРУГИЕ
button-add-game = Добавить игру
button-continue = Продолжить
button-cancel = Отменить
button-cancelling = Отменяю...
button-okay = Хорошо
button-select-all = Выбрать все
button-deselect-all = Снять все
button-enable-all = Включить все
button-disable-all = Отключить все
button-customize = Настроить
button-exit = Выйти
button-comment = Комментарий
button-lock = Заблокировать
button-unlock = Разблокировать
# This opens a download page.
button-get-app = Получить { $app }
button-validate = Проверить
button-override-manifest = Переопределить манифест
button-extend-manifest = Расширить манифест
button-sort = Сортировать
button-download = Скачать
button-upload = Загрузить
button-ignore = Игнорировать
no-roots-are-configured = Добавьте несколько корней для резервирования еще больше данных.
config-is-invalid = Ошибка: неверный файл конфигурации.
manifest-is-invalid = Ошибка: неверный файл конфигурации (манифеста).
manifest-cannot-be-updated = Ошибка: Невозможно проверить обновление файла манифеста. Подключение к Интернету отключено?
cannot-prepare-backup-target = Ошибка: Не удается подготовить резервную копию цели (либо создание, либо очистка папки). Если папка открыта в браузере файлов, попробуйте закрыть её: { $path }
restoration-source-is-invalid = Ошибка: Источник восстановления недействителен (либо не существует или не является каталогом). Пожалуйста, проверьте путь: { $path }
registry-issue = Ошибка: Некоторые записи реестра были пропущены.
unable-to-browse-file-system = Ошибка: Невозможна навигация по файловой системе.
unable-to-open-directory = Ошибка: Не удается открыть каталог:
unable-to-open-url = Ошибка: Не удается открыть URL:
unable-to-configure-cloud = Не удалось настроить облако.
unable-to-synchronize-with-cloud = Не удалось синхронизировать с облаком.
cloud-synchronize-conflict = Ваши локальные и облачные резервные копии конфликтуют. Выполните закачку или загрузку, чтобы разрешить это.
command-unlaunched = Команда не запущена: { $command }
command-terminated = Команда прервана: { $command }
command-failed = Не удалось выполнить команду с кодом { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] игр(а)
       *[other] игр(ы)
    }
processed-games-subset =
    { $processed-games } из { $total-games } { $total-games ->
        [one] игр(а)
       *[other] игр(ы)
    }
processed-size-subset = { $processed-size } из { $total-size }
field-backup-target = Резервировать в:
field-restore-source = Восстановить из:
field-custom-files = Пути:
field-custom-registry = Реестр:
field-sort = Сортировать:
field-redirect-source =
    .placeholder = Источник (исходное место)
field-redirect-target =
    .placeholder = Целевой (новое место)
field-roots = Корневые:
field-backup-excluded-items = Исключения из резервной копии:
field-redirects = Перенаправления:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Полный:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Дифференциальный:
field-backup-format = Формат:
field-backup-compression = Сжатие:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Степень:
label-manifest = Манифест
# This shows the time when we checked for an update to the manifest.
label-checked = Проверен
# This shows the time when we found an update to the manifest.
label-updated = Обновлено
label-new = Новый
label-removed = Удалено
label-comment = Комментарий
label-unchanged = Без изменений
label-backup = Резервировать
label-scan = Сканирование
label-filter = Фильтр
label-unique = Уникальный
label-complete = Полный
label-partial = Частичный
label-enabled = Включено
label-disabled = Отключено
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Потоки
label-cloud = Облачное хранилище
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Удалённый репозиторий
label-remote-name = Имя репозитория
label-folder = Папка
# An executable file
label-executable = Исполняемый файл
# Options given to a command line program
label-arguments = Аргументы
label-url = Ссылка
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Хост
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Порт
label-username = Имя пользователя
label-password = Пароль
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Хранилище
label-custom = Пользовательское
label-none = Нет
label-change-count = Изменений: { $total }
label-unscanned = Непроверенные
# This refers to a local file on the computer
label-file = Файл
label-game = Игра
# Aliases are alternative titles for the same game.
label-alias = Псевдоним (похожее)
label-original-name = Исходное название
# Which manifest a game's data came from
label-source = Источник
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Основной манифест
# This refers to how we integrate a custom game with the manifest data.
label-integration = Интеграция
# This is a folder name where a specific game is installed
label-installed-name = Установленное имя
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
store-other-home = Домашняя папка
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine префикс
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows диск
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux диск
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac диск
store-other = Другое
backup-format-simple = Простой
backup-format-zip = Zip
compression-none = Нет
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Дефляция
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Тема оформления
theme-light = Светлая
theme-dark = Тёмная
redirect-bidirectional = Двунаправленный
reverse-redirects-when-restoring = Обратная последовательность перенаправлений при восстановлении
show-disabled-games = Показать неактивные игры
show-unchanged-games = Показать неизменные игры
show-unscanned-games = Показать несканированные игры
override-max-threads = Переопределить макс. количество потоков
synchronize-automatically = Синхронизировать автоматически
prefer-alias-display = Отображать своё название вместо исходного
skip-unconstructive-backups = Пропустить резервную копию, когда данные будут удалены, но не добавлены или обновлены
explanation-for-exclude-store-screenshots = В резервных копиях исключить скриншоты из конкретного магазина
explanation-for-exclude-cloud-games = Не создавать резервные копии игр с поддержкой облака на этих платформах
consider-doing-a-preview =
    Если вы еще этого не сделали, предлагаю сначала сделать предварительный просмотр, чтобы
    не было сюрпризов.
confirm-backup =
    Вы уверены, что хотите продолжить создание резервной копии? { $path-action ->
        [merge] Новые данные сохранения будут объединены в целевую папку:
       *[create] Будет создана целевая папка:
    }
confirm-restore =
    Вы уверены, что хотите продолжить восстановление?
    Это перезапишет все текущие файлы резервными копиями отсюда:
confirm-cloud-upload =
    Вы хотите заменить ваши облачные файлы локальными файлами?
    Ваши файлы в облаке ({ $cloud-path }) станут точной копией локальных файлов ({ $local-path }).
    Файлы в облаке будут обновлены или удалены по мере необходимости.
confirm-cloud-download =
    Вы хотите заменить локальные файлы на ваши облачные файлы?
    Ваши локальные файлы ({ $local-path }) станут точной копией ваших облачных файлов ({ $cloud-path }).
    По мере необходимости локальные файлы будут обновлены или удалены.
confirm-add-missing-roots = Добавить эти корневые папки?
no-missing-roots = Дополнительные корневые папки не найдены.
loading = Загрузка...
preparing-backup-target = Подготовка папки резервной копии...
updating-manifest = Обновление манифеста...
no-cloud-changes = Нет изменений для синхронизации
backups-are-valid = Ваши резервные копии действительны.
backups-are-invalid =
    Резервные копии этих игр кажутся недействительными.
    Вы хотите создать новые полные резервные копии для этих игр?
saves-found = Найдены данные сохранения.
no-saves-found = Сохраненных данных не найдено.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = без подтверждения
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = Требуется перезапуск
prefix-error = Ошибка: { $message }
prefix-warning = Внимание: { $message }
cloud-app-unavailable = Облачные резервные копии отключены, потому что { $app } недоступно.
cloud-not-configured = Облачные резервные копии отключены, так как облачная система не настроена.
cloud-path-invalid = Облачные резервные копии отключены, так как путь резервного копирования недействителен.
game-is-unrecognized = Ludusavi не распознает эту игру.
game-has-nothing-to-restore = У этой игры нет резервной копии для восстановления.
launch-game-after-error = Все равно запустить игру?
game-did-not-launch = Не удалось запустить игру.
backup-is-newer-than-current-data = Существующая резервная копия новее, чем текущая.
backup-is-older-than-current-data = Существующая резервная копия старше, чем текущая.
back-up-specific-game =
    .confirm = Создать резервную копию для { $game }?
    .failed = Не удалось сохранить данные для { $game }
restore-specific-game =
    .confirm = Восстановить сохраненные данные для { $game }?
    .failed = Не удалось восстановить данные для { $game }
new-version-check = Автоматически проверять обновления приложения
new-version-available = Доступно обновление приложения: { $version }. Хотите просмотреть список изменений (патчноут)?
custom-game-will-override = Пользовательская игра переопределяет элемент манифеста
custom-game-will-extend = Пользовательская игра расширяет манифест
operation-will-only-include-listed-games = Обработаются только перечисленные игры

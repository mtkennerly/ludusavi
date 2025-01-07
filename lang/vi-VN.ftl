ludusavi = Ludusavi
language = Ngôn ngữ
game-name = Tên
total-games = Trò Chơi
file-size = Kích thước
file-location = Vị trí
overall = Tổng thể
status = Trạng thái
cli-unrecognized-games = No info for these games:
cli-unable-to-request-confirmation = Unable to request confirmation.
    .winpty-workaround = If you are using a Bash emulator (like Git Bash), try running winpty.
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.
badge-failed = THẤT BẠI
badge-duplicates = DUPLICATES
badge-duplicated = DUPLICATED
badge-ignored = IGNORED
badge-redirected-from = FROM: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = Some entries failed to process; look for { badge-failed } in the output for details. Double check whether you can access those files or whether their paths are very long.
cli-game-line-item-redirected = Redirected from: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
button-backup = Sao lưu
button-preview = Xem trước
button-restore = Khôi phục
button-nav-backup = BACKUP MODE
button-nav-restore = RESTORE MODE
button-nav-custom-games = CUSTOM GAMES
button-nav-other = OTHER
button-add-game = Thêm trò chơi
button-continue = Tiếp tục
button-cancel = Hủy
button-cancelling = Hủy bỏ...
button-okay = Đồng ý
button-select-all = Chọn tất cả
button-deselect-all = Bỏ chọn tất cả
button-enable-all = Bật toàn bộ
button-disable-all = Tắt toàn bộ
button-customize = Tùy chỉnh
button-exit = Thoát
button-comment = Ghi chú
button-lock = Khoá
button-unlock = Mở khoá
# This opens a download page.
button-get-app = Get { $app }
button-validate = Validate
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Phân loại
no-roots-are-configured = Add some roots to back up even more data.
config-is-invalid = Error: The config file is invalid.
manifest-is-invalid = Error: The manifest file is invalid.
manifest-cannot-be-updated = Error: Unable to check for an update to the manifest file. Is your Internet connection down?
cannot-prepare-backup-target = Error: Unable to prepare backup target (either creating or emptying the folder). If you have the folder open in your file browser, try closing it: { $path }
restoration-source-is-invalid = Error: The restoration source is invalid (either doesn't exist or isn't a directory). Please double check the location: { $path }
registry-issue = Error: Some registry entries were skipped.
unable-to-browse-file-system = Error: Unable to browse on your system.
unable-to-open-directory = Error: Unable to open directory:
unable-to-open-url = Error: Unable to open URL:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
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
field-backup-target = Back up to:
field-restore-source = Phục hồi từ:
field-custom-files = Paths:
field-custom-registry = Registry:
field-sort = Sort:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-roots = Roots:
field-backup-excluded-items = Backup exclusions:
field-redirects = Chuyển đến:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Định dạng:
field-backup-compression = Nén:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Mức độ:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Đã kiểm tra
# This shows the time when we found an update to the manifest.
label-updated = Đã cập nhật
label-new = Mới
label-removed = Đã gỡ bỏ
label-comment = Ghi chú
label-unchanged = Chưa thay đổi
label-backup = Sao lưu
label-scan = Quét
label-filter = Bộ lọc
label-unique = Unique
label-complete = Hoàn thành
label-partial = Partial
label-enabled = Kích hoạt
label-disabled = Vô hiệu hóa
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Chủ đề
label-cloud = Đám mây
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Điều khiên
label-remote-name = Tên máy chủ
label-folder = Thư mục
# An executable file
label-executable = File Thực thi
# Options given to a command line program
label-arguments = Arguments
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Host
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Cổng
label-username = Tên tài khoản
label-password = Mật khẩu
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Bên cung cấp
label-custom = Tuỳ chỉnh
label-none = Trống
label-change-count = Changes: { $total }
label-unscanned = Không quét
# This refers to a local file on the computer
label-file = Tệp
label-game = Trò chơi
# Aliases are alternative titles for the same game.
label-alias = Biệt hiệu
label-original-name = Tên gốc
# Which manifest a game's data came from
label-source = Nguồn
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primary manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integration
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
store-other-home = Home folder
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine prefix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows drive
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux drive
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac drive
store-other = Other
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = None
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Theme
theme-light = Light
theme-dark = Dark
redirect-bidirectional = Bidirectional
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
prefer-alias-display = Display alias instead of original name
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
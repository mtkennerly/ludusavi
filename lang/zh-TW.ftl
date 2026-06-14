ludusavi = Ludusavi
language = 語言
game-name = 名稱
total-games = 遊戲
file-size = 尺寸
file-location = 位置
overall = 總覽
status = 狀態
cli-unrecognized-games = 沒有這些遊戲的資訊：
cli-unable-to-request-confirmation = 無法請求確認。
    .winpty-workaround = 如果您使用的是 Bash 模擬器（例如 Git Bash），請嘗試執行 winpty。
cli-backup-id-with-multiple-games = 恢復多個遊戲時無法指定備份ID。
cli-invalid-backup-id = 無效的備份ID。
badge-failed = 已失敗
badge-duplicates = 重複項目
badge-duplicated = 已重複
badge-ignored = 忽略
badge-redirected-from = 來自： { $path }
badge-redirecting-to = 到： { $path }
some-entries-failed = 某些條目處理失敗；在輸出中查找 { badge-failed } 以獲取詳細資訊。請仔細檢查您是否可以存取這些文件或它們的路徑是否太長。
cli-game-line-item-redirected = 重新導向從: { $path }
cli-game-line-item-redirecting = 重新導向到：{ $path }
button-backup = 備份
button-preview = 預覽
button-restore = 復原
button-nav-backup = 備份模式
button-nav-restore = 還原模式
button-nav-custom-games = 自訂遊戲
button-nav-other = 其他
button-add-game = 新增遊戲
button-continue = 繼續
button-cancel = 取消
button-cancelling = 正在取消...
button-okay = 確定
button-select-all = 選取全部
button-deselect-all = 全不選
button-enable-all = 啟用全部
button-disable-all = 停用全部
button-customize = 自定義
button-exit = 退出
button-comment = 備註
button-lock = 鎖定
button-unlock = 解鎖
# This opens a download page.
button-get-app = 取得 { $app }
button-validate = 驗證
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sort
button-download = Download
button-upload = Upload
button-ignore = Ignore
no-roots-are-configured = 增加其他根目錄，可以備份更多資料。
config-is-invalid = 錯誤：設定檔無效。
manifest-is-invalid = 錯誤：manifest 清單檔無效。
manifest-cannot-be-updated = 錯誤：無法檢查 manifest 清單文件的更新。您的網路是否已斷線？
cannot-prepare-backup-target = 錯誤：無法準備備份目標（無法新增或清空資料夾）。如果您打開了該資料夾，請嘗試關閉它：{ $path }
restoration-source-is-invalid = 錯誤：還原來源無效（路徑不存在或不是目錄）。請重新檢查位置：{ $path }
registry-issue = 錯誤：某些註冊表被跳過。
unable-to-browse-file-system = 錯誤：無法在您的系統上瀏覽。
unable-to-open-directory = 錯誤：無法打開資料夾：
unable-to-open-url = 錯誤：無法打開網址：
unable-to-configure-cloud = 無法設定雲端。
unable-to-synchronize-with-cloud = 無法同步至雲端。
cloud-synchronize-conflict = 您的本地備份和雲端備份有衝突。請進行上傳或下載以解決此問題。
command-unlaunched = 命令未啟動：{ $command }
command-terminated = 命令已中斷：{ $command }
command-failed = 命令失敗，錯誤碼 { $code }：{ $command }
processed-games =
    { $total-games } { $total-games ->
        [one] 遊戲
       *[other] 遊戲
    }
processed-games-subset =
    { $processed-games } / { $total-games } { $total-games ->
        [one] 遊戲
       *[other] 遊戲
    }
processed-size-subset = { $processed-size } / { $total-size }
field-backup-target = 備份至:
field-restore-source = 還原自：
field-custom-files = 路徑：
field-custom-registry = 註冊表：
field-sort = 排序：
field-redirect-source =
    .placeholder = 來源（原始位置）
field-redirect-target =
    .placeholder = 目標（新位置）
field-roots = 根目錄：
field-backup-excluded-items = 備份排除：
field-redirects = 路徑重定向：
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = 完整備份：
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = 差異備份：
field-backup-format = 格式：
field-backup-compression = 壓縮：
# The compression level determines how much compresison we perform.
field-backup-compression-level = 壓縮等級：
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = 已檢查
# This shows the time when we found an update to the manifest.
label-updated = 已更新
label-new = 最新
label-removed = 已移除
label-comment = 註釋
label-unchanged = 未變更
label-backup = Backup
label-scan = 掃描
label-filter = 篩選
label-unique = 唯一
label-complete = 已完成
label-partial = 部分
label-enabled = 已啟用
label-disabled = 已停用
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = 執行緒
label-cloud = 雲端備份
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = 遠端
label-remote-name = 遠端名稱
label-folder = 資料夾
# An executable file
label-executable = 可執行文件
# Options given to a command line program
label-arguments = 參數
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = 主機
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = 端口
label-username = 使用者名稱
label-password = 密碼
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = 供應商
label-custom = 自定義
label-none = 無
label-change-count = 變更：{ $total }
label-unscanned = 未掃描
# This refers to a local file on the computer
label-file = 檔案
label-game = 遊戲
# Aliases are alternative titles for the same game.
label-alias = 別名
label-original-name = 原始名稱
# Which manifest a game's data came from
label-source = Source
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Primary manifest
# This refers to how we integrate a custom game with the manifest data.
label-integration = Integration
# This is a folder name where a specific game is installed
label-installed-name = Installed name
store-ea = EA
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-legendary = Legendary
store-lutris = Lutris
store-microsoft = 微軟
store-origin = Origin
store-prime = Amazon Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = 主要資料夾
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
store-other = 其它
backup-format-simple = 簡易
backup-format-zip = Zip檔
compression-none = 無
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = 外觀主題
theme-light = 亮色主題
theme-dark = 暗色主題
redirect-bidirectional = 雙向
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = 顯示未變更的遊戲
show-unscanned-games = 顯示未掃描的遊戲
override-max-threads = 覆蓋最大執行緒數
synchronize-automatically = 自動同步
prefer-alias-display = 顯示別名而非原始名稱
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = 在備份中，排除特定的商店截圖
explanation-for-exclude-cloud-games = 不備份這些平台上內建雲端儲存的遊戲
consider-doing-a-preview = 如果尚未進行預覽，建議先做一次預覽，以免有意外情況。
confirm-backup =
    您確定要繼續備份嗎？ { $path-action ->
        [merge] 新的存檔將合併到目標資料夾中：
       *[create] 目標資料夾將被創建：
    }
confirm-restore =
    您確定要繼續還原嗎？
    這將會用來自以下位置的備份覆蓋當前的檔案：
confirm-cloud-upload =
    您是否要用本地文件替換雲端存檔？
    您的雲端存檔（{ $cloud-path }）將成為本地存檔（{ $local-path }）的副本。
    雲端中的檔案將會在需要的時候進行更新或刪除。
confirm-cloud-download =
    您是否要用雲端存檔替換本地存檔？
    您的本地存檔（{ $local-path }）將成為雲端存檔（{ $cloud-path }）的副本。
    本地檔案將會在需要的時候進行更新或刪除。
confirm-add-missing-roots = 增加這些根目錄嗎？
no-missing-roots = 未找到其他根目錄。
loading = 讀取中...
preparing-backup-target = 正在準備備份資料夾...
updating-manifest = 正在更新 manifest 清單檔...
no-cloud-changes = 沒有需要同步的修改
backups-are-valid = 您的備份是有效的。
backups-are-invalid =
    這些遊戲的備份似乎無效。
    您是否要為這些遊戲創建新的完整備份？
saves-found = 已找到存檔資料。
no-saves-found = 沒有找到任何存檔。
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = 不進行確認
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = 需要重新啟動
prefix-error = 錯誤：{ $message }
prefix-warning = 警告：{ $message }
cloud-app-unavailable = 雲端備份已禁用，因為 { $app } 不可用。
cloud-not-configured = 雲端備份已被禁用，因為沒有設定任何雲端系統。
cloud-path-invalid = 雲端備份已被禁用，因為備份路徑無效。
game-is-unrecognized = Ludusavi 無法識別此遊戲。
game-has-nothing-to-restore = 此遊戲沒有可還原的備份。
launch-game-after-error = 仍要啟動遊戲嗎？
game-did-not-launch = 遊戲啟動失敗。
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = 要備份 { $game } 的存檔資料嗎？
    .failed = 無法備份 { $game } 的存檔資料
restore-specific-game =
    .confirm = 還原 { $game } 的存檔資料嗎？
    .failed = 無法還原 { $game } 的存檔資料
new-version-check = 自動檢查程式更新
new-version-available = 可用的更新：{ $version }。您要查看更新說明嗎？
custom-game-will-override = This custom game overrides a manifest entry
custom-game-will-extend = This custom game extends a manifest entry
operation-will-only-include-listed-games = This will only process the games that are currently listed

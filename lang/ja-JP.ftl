ludusavi = Ludusavi
language = 言語
game-name = 名前
total-games = ゲーム
file-size = サイズ
file-location = 場所
overall = Overall
status = Status
cli-unrecognized-games = これらのゲームに関する情報はありません：
cli-unable-to-request-confirmation = 確認を要求できません。
    .winpty-workaround = Bashエミュレータ(Git Bashなど)を使用している場合は、winptyを実行してみてください。
cli-backup-id-with-multiple-games = 複数ゲームの復元時に、バックアップIDを指定できない。
cli-invalid-backup-id = 無効なバックアップIDです。
badge-failed = 失敗
badge-duplicates = 重複している
badge-duplicated = 重複済み
badge-ignored = 無効
badge-redirected-from = FROM: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = いくつかのエントリーは処理に失敗しました。詳細は出力にある { badge-failed } をご覧ください。これらのファイルにアクセスできるか、またはそのパスが非常に長いかどうかを再確認してください。
cli-game-line-item-redirected = Redirected from: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
button-backup = バックアップ
button-preview = プレビュー
button-restore = 復元
button-nav-backup = バックアップモード
button-nav-restore = 復元モード
button-nav-custom-games = カスタムゲーム
button-nav-other = その他
button-add-game = ゲームを追加
button-continue = 続ける
button-cancel = キャンセル
button-cancelling = キャンセル中...
button-okay = OK
button-select-all = すべて選択
button-deselect-all = 選択を全て解除
button-enable-all = 全て有効
button-disable-all = 全て無効
button-customize = カスタマイズ
button-exit = 終了
button-comment = Comment
button-lock = Lock
button-unlock = Unlock
# This opens a download page.
button-get-app = Get { $app }
button-validate = 検証
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sort
button-download = Download
button-upload = Upload
button-ignore = Ignore
no-roots-are-configured = いくつかのルートを追加して、さらに多くのデータをバックアップします。
config-is-invalid = エラー：設定ファイルが無効です。
manifest-is-invalid = エラー: マニフェストファイルが無効です。
manifest-cannot-be-updated = エラー：マニフェストファイルの更新を確認できません。インターネット接続が切断されていますか？
cannot-prepare-backup-target = エラー: バックアップ先の準備に失敗しました (フォルダーを作成または空にします)。 ファイル ブラウザでこのフォルダを開いている場合は、閉じてみてください: { $path }
restoration-source-is-invalid = エラー：復元元が無効です(存在しないか、ディレクトリではありません)。場所を再確認してください： { $path }
registry-issue = エラー: 一部のレジストリエントリがスキップされました。
unable-to-browse-file-system = エラー: システム上で参照できません。
unable-to-open-directory = エラー: ディレクトリを開くことができません:
unable-to-open-url = エラー: URLを開くことができません:
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
field-backup-target = バックアップ先:
field-restore-source = 復元元:
field-custom-files = パス:
field-custom-registry = レジストリ:
field-sort = ソート:
field-redirect-source =
    .placeholder = ソース (元の場所)
field-redirect-target =
    .placeholder = ターゲット (新しい場所)
field-roots = ルート:
field-backup-excluded-items = バックアップから除外:
field-redirects = リダイレクト:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = フルバックアップ:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = 差分バックアップ:
field-backup-format = フォーマット:
field-backup-compression = 圧縮:
# The compression level determines how much compresison we perform.
field-backup-compression-level = 圧縮率:
label-manifest = マニフェスト
# This shows the time when we checked for an update to the manifest.
label-checked = チェック
# This shows the time when we found an update to the manifest.
label-updated = 更新日時
label-new = 新しい
label-removed = 削除済み
label-comment = コメント
label-unchanged = 未変更
label-backup = Backup
label-scan = スキャン
label-filter = 絞り込み
label-unique = 単一
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
label-unscanned = Unscanned
# This refers to a local file on the computer
label-file = ファイル
label-game = ゲーム
# Aliases are alternative titles for the same game.
label-alias = エイリアス
label-original-name = 元の名前
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
backup-format-simple = 簡単
backup-format-zip = Zip
compression-none = なし
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = テーマ
theme-light = ライト
theme-dark = ダーク
redirect-bidirectional = 双方向
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = 変更されていないゲームを表示
show-unscanned-games = スキャンされていないゲームを表示
override-max-threads = 使用するCPUの最大スレッド数を上書き
synchronize-automatically = Synchronize automatically
prefer-alias-display = Display alias instead of original name
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = In backups, exclude store-specific screenshots
explanation-for-exclude-cloud-games = 以下のプラットフォームでは、クラウドゲームのバックアップを行いません
consider-doing-a-preview = まだ行っていない場合は、予期しない結果を防ぐためにプレビューを行うことをおすすめします。
confirm-backup =
    Are you sure you want to proceed with the backup? { $path-action ->
        [merge] New save data will be merged into the target folder:
       *[create] The target folder will be created:
    }
confirm-restore =
    復元を続行してもよろしいですか？
    現在のファイルはここから上書きされます:
confirm-cloud-upload =
    Do you want to replace your cloud files with your local files?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to replace your local files with your cloud files?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = このルートを追加しますか?
no-missing-roots = 追加するルートが見つかりませんでした。
loading = ロード中...
preparing-backup-target = バックアップディレクトリを準備中...
updating-manifest = マニフェストを更新中...
no-cloud-changes = No changes to synchronize
backups-are-valid = バックアップは正常です。
backups-are-invalid =
    これらゲームのバックアップは無効のようです。
    フルバックアップを新規作成しますか？
saves-found = セーブデータが見つかりました。
no-saves-found = セーブデータが見つかりませんでした。
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = 確認なし
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = 再起動が必要
prefix-error = エラー： { $message }
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

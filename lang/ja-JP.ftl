ludusavi = Ludusavi
language = 言語
font = フォント
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
badge-duplicates = DUPLICATES
badge-duplicated = DUPLICATED
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
toggle-backup-merge = マージ
field-restore-source = 復元元:
field-custom-files = パス:
field-custom-registry = レジストリ:
field-sort = ソート:
field-redirect-source =
    .placeholder = ソース (元の場所)
field-redirect-target =
    .placeholder = ターゲット (新しい場所)
field-roots = Roots:
field-backup-excluded-items = バックアップから除外:
field-redirects = Redirects:
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
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home folder
store-other-wine = Wine prefix
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
redirect-bidirectional = Bidirectional
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
explanation-for-exclude-store-screenshots =
    In backups, exclude store-specific screenshots. Right now, this only applies
    to { store-steam } screenshots that you've taken. If a game has its own built-in
    screenshot functionality, this setting will not affect whether those
    screenshots are backed up.
consider-doing-a-preview = まだ行っていない場合は、予期しない結果を防ぐためにプレビューを行うことをおすすめします。
confirm-backup =
    Are you sure you want to proceed with the backup? { $path-action ->
        [merge] New save data will be merged into the target folder:
        [recreate] The target folder will be deleted and recreated from scratch:
       *[create] The target folder will be created:
    }
confirm-restore =
    復元を続行してもよろしいですか？
    現在のファイルはここから上書きされます:
confirm-add-missing-roots = このルートを追加しますか?
no-missing-roots = 追加するルートが見つかりませんでした。
preparing-backup-target = バックアップディレクトリを準備中...
updating-manifest = マニフェストを更新中...
saves-found = セーブデータが見つかりました。
no-saves-found = セーブデータが見つかりませんでした。
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = 確認なし
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required

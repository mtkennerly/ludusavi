ludusavi = Ludusavi
language = 语言
game-name = 名称
total-games = 游戏
file-size = 大小
file-location = 位置
overall = 总体
status = Status
cli-unrecognized-games = 没有这些游戏的信息：
cli-unable-to-request-confirmation = 无法请求确认。
    .winpty-workaround = 若您正在使用 Bash 模拟器（例如 Git Bash），请尝试运行 winpty。
cli-backup-id-with-multiple-games = 恢复多个游戏时无法指定备份 ID。
cli-invalid-backup-id = 无效的备份 ID。
badge-failed = 已失败
badge-duplicates = 复制为副本
badge-duplicated = 已复制为副本
badge-ignored = 已忽略
badge-redirected-from = 来自：{ $path }
badge-redirecting-to = 到： { $path }
some-entries-failed = 有些条目无法处理；详情请参阅输出中的 { badge-failed }。请仔细检查您是否可以访问这些文件，或者它们的路径是否太长。
cli-game-line-item-redirected = 重定向自：{ $path }
cli-game-line-item-redirecting = 重定向到：{ $path }
button-backup = 备份
button-preview = 预览
button-restore = 恢复
button-nav-backup = 备份模式
button-nav-restore = 恢复模式
button-nav-custom-games = 自定义游戏
button-nav-other = 其他
button-add-game = 添加游戏
button-continue = 继续
button-cancel = 取消
button-cancelling = 取消中...
button-okay = 好的
button-select-all = 全选
button-deselect-all = 全不选
button-enable-all = 全部启用
button-disable-all = 全部禁用
button-customize = 自定义
button-exit = 退出
button-comment = Comment
button-lock = Lock
button-unlock = Unlock
# This opens a download page.
button-get-app = Get { $app }
button-validate = 验证
no-roots-are-configured = 添加一些根，以备份甚至更多的数据。
config-is-invalid = 错误：配置文件无效。
manifest-is-invalid = 错误：manifest 文件无效。
manifest-cannot-be-updated = 错误：无法检查 manifest 文件的更新。您的互联网连接是否已断开？
cannot-prepare-backup-target = 错误：无法准备备份目标（创建或清空文件夹）。若您在文件浏览器中打开了该文件夹，请尝试关闭它：{ $path }
restoration-source-is-invalid = 错误：恢复源无效（不存在或非目录）。请仔细检查位置：{ $path }
registry-issue = 错误：一些注册表条目被跳过。
unable-to-browse-file-system = 错误：无法浏览您的系统。
unable-to-open-directory = 错误：无法打开目录：
unable-to-open-url = 错误：无法打开链接：
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games = { $total-games } 游戏
processed-games-subset = { $processed-games }，共 { $total-games } 游戏
processed-size-subset = { $total-size }中的{ $processed-size }
field-backup-target = 备份到:
field-restore-source = 还原自
field-custom-files = 路径：
field-custom-registry = 注册表
field-sort = 排序：
field-redirect-source =
    .placeholder = 源 (原始位置)
field-redirect-target =
    .placeholder = 目标 (新位置)
field-roots = Roots:
field-backup-excluded-items = 备份排除：
field-redirects = 文件夹重定向
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = 完整备份
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = 差异备份
field-backup-format = 格式：
field-backup-compression = 压缩：
# The compression level determines how much compresison we perform.
field-backup-compression-level = 压缩等级：
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = 已检查
# This shows the time when we found an update to the manifest.
label-updated = 已更新
label-new = 新的存档
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
store-ea = EA
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-lutris = Lutris
store-microsoft = 微软商店
store-origin = Origin
store-prime = 亚马逊 Prime Gaming
store-steam = Steam
store-uplay = 育碧 Uplay
store-other-home = 主文件夹
store-other-wine = Wine prefix
store-other = 其他
backup-format-simple = 普通
backup-format-zip = Zip 文件
compression-none = 不进行压缩
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = 主题
theme-light = 亮
theme-dark = 暗
redirect-bidirectional = 双向
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots = 在备份中，排除特定商店的屏幕截图
consider-doing-a-preview = 如果您还没有预览，请考虑先进行预览，防止发生任何意料之外的结果。
confirm-backup =
    确定要继续备份吗？ { $path-action ->
        [merge] 新保存的数据将被合并到目标文件夹中：
       *[create] 目标文件夹将被创建：
    }
confirm-restore =
    您确定要继续恢复吗？
    这将会覆盖当前备份的所有文件：
confirm-cloud-upload =
    Do you want to replace your cloud files with your local files?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to replace your local files with your cloud files?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = 添加这些根目录吗？
no-missing-roots = 未找到其他根目录。
loading = Loading...
preparing-backup-target = 正在准备备份文件夹...
updating-manifest = 正在更新 Manifest 文件...
no-cloud-changes = No changes to synchronize
backups-are-valid = 您的备份是有效的。
backups-are-invalid =
    这些游戏的备份似乎无效。
    您想为这些游戏创建新的完全备份吗？
saves-found = 发现已有的存档。
no-saves-found = 未找到存档。
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = 不进行确认
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

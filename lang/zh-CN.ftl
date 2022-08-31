ludusavi = 录读加一
language = 语言
font = 字体
game-name = Name
total-games = 游戏
file-size = 大小
file-location = 位置
overall = 总体
cli-backup-target-already-exists = 备份目标已存在于 ( { $path } )。要么选择一个不同的 --path 参数，要么使用 --force 参数删除它。
cli-unrecognized-games = 没有这些游戏的信息：
cli-confirm-restoration = 您想从 { $path } 恢复吗？
cli-unable-to-request-confirmation = 无法请求确认。
    .winpty-workaround = 若您正在使用 Bash 模拟器（例如 Git Bash），请尝试运行 winpty。
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.
badge-failed = 已失败
badge-duplicates = 复制为副本
badge-duplicated = 已复制为副本
badge-ignored = 已忽略
badge-redirected-from = 来自：{ $path }
some-entries-failed = 有些条目无法处理；详情请参阅输出中的 { badge-failed }。请仔细检查您是否可以访问这些文件，或者它们的路径是否太长。
cli-game-line-item-redirected = 重定向自：{ $path }
button-backup = 备份
button-preview = 预览
button-restore = 恢复
button-nav-backup = 备份模式
button-nav-restore = 恢复模式
button-nav-custom-games = 自定义游戏
button-nav-other = 其他
button-add-root = 添加根
button-find-roots = 寻找根
button-add-redirect = 添加重定向
button-add-game = 添加游戏
button-continue = 继续
button-cancel = 取消
button-cancelling = 取消中...
button-okay = 好的
button-select-all = 全选
button-deselect-all = 全不选
button-enable-all = 全部启用
button-disable-all = 全部禁用
button-customize = Customize
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
processed-games = { $total-games } 游戏
processed-games-subset = { $processed-games }，共 { $total-games } 游戏
processed-size-subset = { $total-size }中的{ $processed-size }
field-backup-target = 备份到:
toggle-backup-merge = 合并
field-restore-source = 还原自
field-custom-files = 路径：
field-custom-registry = 注册表
field-search = 搜索:
field-sort = 排序：
field-redirect-source =
    .placeholder = 源 (原始位置)
field-redirect-target =
    .placeholder = 目标 (新位置)
field-backup-excluded-items = 备份排除：
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = 完整备份
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = 差异备份
field-backup-format = 格式：
field-backup-compression = 压缩：
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = 主文件夹
store-other-wine = Wine prefix
store-other = Other
sort-reversed = Reversed
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = None
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = 主题
theme-light = 亮
theme-dark = 暗
explanation-for-exclude-other-os-data =
    In backups, exclude save locations that have only been confirmed on another
    operating system. Some games always put saves in the same place, but the
    locations may have only been confirmed for a different OS, so it can help
    to check them anyway. Excluding that data may help to avoid false positives,
    but may also mean missing out on some saves. On Linux, Proton saves will
    still be backed up regardless of this setting.
explanation-for-exclude-store-screenshots =
    In backups, exclude store-specific screenshots. Right now, this only applies
    to { store-steam } screenshots that you've taken. If a game has its own built-in
    screenshot functionality, this setting will not affect whether those
    screenshots are backed up.
consider-doing-a-preview = 如果您还没有预览，请考虑先进行预览，防止发生任何意料之外的结果。
confirm-backup =
    Are you sure you want to proceed with the backup? { $path-action ->
        [merge] New save data will be merged into the target folder:
        [recreate] The target folder will be deleted and recreated from scratch:
       *[create] The target folder will be created:
    }
confirm-restore =
    Are you sure you want to proceed with the restoration?
    This will overwrite any current files with the backups from here:
confirm-add-missing-roots = Add these roots?
no-missing-roots = No additional roots found.
preparing-backup-target = Preparing backup directory...

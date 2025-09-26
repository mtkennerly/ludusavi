ludusavi = Ludusavi
language = 语言
game-name = 名称
total-games = 游戏
file-size = 大小
file-location = 位置
overall = 总体
status = 状态
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
button-comment = 备注
button-lock = 锁定
button-unlock = 解锁
# This opens a download page.
button-get-app = 获取 { $app }
button-validate = 验证
button-override-manifest = 覆盖清单
button-extend-manifest = 扩展清单
button-sort = 排序
button-download = 下载
button-upload = 上传
button-ignore = 忽略
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
unable-to-configure-cloud = 无法配置云备份
unable-to-synchronize-with-cloud = 无法与云备份同步
cloud-synchronize-conflict = 你的本地文件和云备份发生冲突，执行上传或下载以解决这个问题。
command-unlaunched = 命令未启动： { $command }
command-terminated = 命令突然终止： { $command }
command-failed = 命令失败，错误代码 { $code }: { $command }
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
field-roots = 根目录：
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
label-manifest = 预设列表
# This shows the time when we checked for an update to the manifest.
label-checked = 已检查
# This shows the time when we found an update to the manifest.
label-updated = 已更新
label-new = 新的存档
label-removed = 删除
label-comment = 备注
label-unchanged = 未改变
label-backup = 备份
label-scan = 扫描
label-filter = 筛选
label-unique = 单一文件
label-complete = 全部
label-partial = 部分
label-enabled = 启用
label-disabled = 禁用
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = 线程
label-cloud = 云备份
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = 远程
label-remote-name = 远程名称
label-folder = 目录
# An executable file
label-executable = 可执行文件
# Options given to a command line program
label-arguments = 参数
label-url = 链接地址
# https://en.wikipedia.org/wiki/Host_(network)
label-host = 主机
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = 端口
label-username = 用户名
label-password = 密码
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = 提供方
label-custom = 自定义
label-none = 无
label-change-count = 更改︰ { $total }
label-unscanned = 未扫描
# This refers to a local file on the computer
label-file = 文件
label-game = 游戏
# Aliases are alternative titles for the same game.
label-alias = 别名
label-original-name = 原始名称
# Which manifest a game's data came from
label-source = 游戏数据来源
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = 主要清单
# This refers to how we integrate a custom game with the manifest data.
label-integration = 集成方式
# This is a folder name where a specific game is installed
label-installed-name = 安装名称
store-ea = EA
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-legendary = Legendary
store-lutris = Lutris
store-microsoft = 微软商店
store-origin = Origin
store-prime = 亚马逊 Prime Gaming
store-steam = Steam
store-uplay = 育碧 Uplay
store-other-home = 主文件夹
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine prefix
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = 其它Windows商店
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = 其它Linux商店
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = 其它Mac商店
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
reverse-redirects-when-restoring = 恢复时反转重定向顺序
show-disabled-games = 显示禁用的游戏
show-unchanged-games = 显示未修改的游戏
show-unscanned-games = 显示未扫描的游戏
override-max-threads = 覆盖最大线程
synchronize-automatically = 自动同步
prefer-alias-display = 显示别名而不是原始名称
skip-unconstructive-backups = 当数据被删除而非添加或更新时，跳过备份
explanation-for-exclude-store-screenshots = 在备份中，排除特定商店的屏幕截图
explanation-for-exclude-cloud-games = 不要在这些平台上备份云支持的游戏
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
    你想要用本地文件替换云备份文件吗？
    云备份文件({ $cloud-path }) 将成为本地文件的副本({ $local-path })。
    云备份的文件将根据需求在必要时更新或删除。
confirm-cloud-download =
    你想要用云备份文件替换本地文件吗？
    本地文件({ $local-path }) 将成为云备份文件的副本({ $cloud-path })。
    本地的文件将根据需求在必要时更新或删除。
confirm-add-missing-roots = 添加这些根目录吗？
no-missing-roots = 未找到其他根目录。
loading = 正在加载...
preparing-backup-target = 正在准备备份文件夹...
updating-manifest = 正在更新 Manifest 文件...
no-cloud-changes = 没有需要同步的更改
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
suffix-restart-required = 需要重启
prefix-error = 错误： { $message }
prefix-warning = 警告： { $message }
cloud-app-unavailable = 云备份已禁用，因为 { $app } 不可用。
cloud-not-configured = 云备份已禁用，因为没有配置云远程设置。
cloud-path-invalid = 云备份已禁用，因为备份路径无效。
game-is-unrecognized = Ludusavi 不能识别此游戏
game-has-nothing-to-restore = 此游戏没有备份可以恢复。
launch-game-after-error = 仍然要启动游戏吗？
game-did-not-launch = 游戏启动失败。
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = 要备份 { $game } 的存档数据吗？
    .failed = 备份 { $game } 的存档数据失败
restore-specific-game =
    .confirm = 要恢复 { $game } 的存档数据吗？
    .failed = 恢复 { $game } 的存档数据失败
new-version-check = 自动检查应用程序更新
new-version-available = 应用程序更新可用：{ $version }. 是否要查看发行说明？
custom-game-will-override = 这个自定义游戏会覆盖一个清单项
custom-game-will-extend = 这个自定义游戏会扩展一个清单项
operation-will-only-include-listed-games = 这将仅处理当前列出的游戏

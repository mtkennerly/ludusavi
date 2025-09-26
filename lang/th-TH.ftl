ludusavi = Ludusavi
language = ภาษา
game-name = ชื่อเกม
total-games = เกม
file-size = ขนาด
file-location = ที่ตั้งไฟล์
overall = ภาพรวม
status = สถานะ
cli-unrecognized-games = ไม่มีข้อมูลสำหรับเกมนี้
cli-unable-to-request-confirmation = ไม่สามารถร้องขอการยืนยันได้
    .winpty-workaround = ถ้าคุณกำลังใช้ Bash emulator (เหมือน Git Bash), ลองรัน winpty
cli-backup-id-with-multiple-games = ไม่สามารถระบุ backup ID เมื่อกำลังคืนค่าหลายเกมได้
cli-invalid-backup-id = Backup ID ไม่ถูกต้อง
badge-failed = ล้มเหลว
badge-duplicates = ซ้ำกัน
badge-duplicated = DUPLICATED
badge-ignored = ละเว้น
badge-redirected-from = จาก: { $path }
badge-redirecting-to = ไปยัง: { $path }
some-entries-failed = Some entries failed to process; look for { badge-failed } in the output for details. Double check whether you can access those files or whether their paths are very long.
cli-game-line-item-redirected = เปลี่ยนเส้นทางจาก: { $path }
cli-game-line-item-redirecting = เปลี่ยนเส้นทางไปยัง: { $path }
button-backup = สำรองข้อมูล
button-preview = แสดงตัวอย่าง
button-restore = คืนค่า
button-nav-backup = โหมดสำรองข้อมูล
button-nav-restore = โหมดคืนค่าข้อมูล
button-nav-custom-games = เกมที่กำหนดเอง
button-nav-other = อื่นๆ
button-add-game = เพิ่มเกม
button-continue = ดำเนินการต่อ
button-cancel = ยกเลิก
button-cancelling = กำลังยกเลิก...
button-okay = ตกลง
button-select-all = เลือกทั้งหมด
button-deselect-all = ไม่เลือกทั้งหมด
button-enable-all = เปิดทั้งหมด
button-disable-all = ปิดทั้งหมด
button-customize = Customize
button-exit = ออก
button-comment = Comment
button-lock = ล็อค
button-unlock = ปลดล็อค
# This opens a download page.
button-get-app = Get { $app }
button-validate = Validate
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sort
button-download = Download
button-upload = Upload
button-ignore = Ignore
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
field-backup-target = สำรองไปยัง:
field-restore-source = คืนค่าจาก:
field-custom-files = Paths:
field-custom-registry = Registry:
field-sort = เรียงตาม:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-roots = Roots:
field-backup-excluded-items = Backup exclusions:
field-redirects = Redirects:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = เต็มรูปแบบ:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = รูปแบบ:
field-backup-compression = การบีบอัด:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Level:
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Checked
# This shows the time when we found an update to the manifest.
label-updated = Updated
label-new = New
label-removed = Removed
label-comment = Comment
label-unchanged = Unchanged
label-backup = Backup
label-scan = สแกน
label-filter = ตัวกรอง
label-unique = Unique
label-complete = สมบูรณ์
label-partial = บางส่วน
label-enabled = เปิดใช้งาน
label-disabled = ปิดใช้งาน
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Remote
label-remote-name = Remote name
label-folder = โฟลเดอร์
# An executable file
label-executable = Executable
# Options given to a command line program
label-arguments = Arguments
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = โฮส
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = พอร์ต
label-username = ชื่อผู้ใช้
label-password = รหัสผ่าน
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Provider
label-custom = Custom
label-none = ไม่มี
label-change-count = Changes: { $total }
label-unscanned = Unscanned
# This refers to a local file on the computer
label-file = File
label-game = Game
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
compression-none = ไม่มี
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = ธีม
theme-light = สว่าง
theme-dark = มืด
redirect-bidirectional = Bidirectional
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = แสดงเกมที่ไม่ได้เปลี่ยนแปลง
show-unscanned-games = แสดงเกมที่ไม่ได้สแกน
override-max-threads = Override max threads
synchronize-automatically = ซิงค์อัตโนมัติ
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
loading = กำลังโหลด...
preparing-backup-target = กำลังเตรียมการสำรอง directory...
updating-manifest = กำลังอัพเดต Manifest
no-cloud-changes = ไม่มีการเปลี่ยนแปลงที่จะซิงค์
backups-are-valid = Your backups are valid.
backups-are-invalid =
    These games' backups appear to be invalid.
    Do you want to create new full backups for these games?
saves-found = พบเซฟเกม
no-saves-found = ไม่พบเซฟเกม
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = no confirmation
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = ต้อง Restart ใหม่
prefix-error = ข้อผิดพลาด : { $message }
prefix-warning = คำเตือน: { $message }
cloud-app-unavailable = การสำรองผ่าน Cloud ถูกปิดใช้งานเพราะ { $app } ไม่พร้อมใช้งาน
cloud-not-configured = การสำรองผ่าน Cloud ถูกปิดใช้งานเพราะไม่มีระบบ Cloud ถูกตั้งค่าไว้
cloud-path-invalid = การสำรองผ่าน Cloud ถูกปิดใช้งานเพราะเส้นทางสำรองข้อมูลไม่ถูกต้อง
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

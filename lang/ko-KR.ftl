ludusavi = Ludusavi
language = 언어
game-name = 이름
total-games = 게임
file-size = 크기
file-location = 위치
overall = 전체
status = Status
cli-unrecognized-games = 다음 게임에 대한 정보가 없습니다:
cli-unable-to-request-confirmation = 확인을 요청할 수 없습니다.
    .winpty-workaround = Git Bash와 같은 Bash 에뮬레이터를 사용 중이라면, winpty를 사용해보세요.
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.
badge-failed = 실패
badge-duplicates = 중복됨
badge-duplicated = 복사됨
badge-ignored = 무시됨
badge-redirected-from = 출처: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = 일부 항목을 처리하지 못했습니다. { badge-failed } 를 참조하세요. 해당 파일에 접근할 수 있는지, 파일의 경로가 너무 길지 않은지 확인하세요.
cli-game-line-item-redirected = 다음에서 리다이렉트되었습니다: { $path }
cli-game-line-item-redirecting = Redirecting to: { $path }
button-backup = 백업
button-preview = 미리보기
button-restore = 복원
button-nav-backup = 백업 모드
button-nav-restore = 복원 모드
button-nav-custom-games = 사용자 지정 게임
button-nav-other = 기타
button-add-game = 게임 추가
button-continue = 계속
button-cancel = 취소
button-cancelling = 취소하는 중...
button-okay = 확인
button-select-all = 모두 선택
button-deselect-all = 모두 선택 해제
button-enable-all = 모두 활성화
button-disable-all = 모두 비활성화
button-customize = Customize
button-exit = Exit
button-comment = Comment
button-lock = Lock
button-unlock = Unlock
# This opens a download page.
button-get-app = Get { $app }
button-validate = Validate
no-roots-are-configured = 최상위 디렉토리를 추가해서 더 많은 데이터를 백업하세요.
config-is-invalid = 오류: 설정 파일이 올바르지 않습니다.
manifest-is-invalid = 오류: 매니페스트 파일이 올바르지 않습니다.
manifest-cannot-be-updated = 오류: 매니페스트 파일에 대한 업데이트를 확인할 수 없습니다. 혹시 인터넷 연결이 끊겨있나요?
cannot-prepare-backup-target = 오류: 백업 대상을 준비할 수 없습니다 (폴더를 생성하거나 비우는 작업). 파일 탐색기로 해당 폴더를 열고 있다면 닫아보세요: { $path }
restoration-source-is-invalid = 에러: 복원 대상이 올바르지 않습니다 (해당 디렉토리가 존재하지 않거나 디렉토리가 아닙니다). 다음 위치가 올바른지 한 번 더 확인해주세요: { $path }
registry-issue = 오류: 일부 레지스트리 항목을 건너뛰었습니다.
unable-to-browse-file-system = 오류: 시스템 탐색을 할 수 없습니다.
unable-to-open-directory = 오류: 다음 디렉토리를 열 수 없습니다:
unable-to-open-url = 오류: 다음 URL을 열 수 없습니다:
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
field-restore-source = Restore from:
field-custom-files = Paths:
field-custom-registry = Registry:
field-sort = Sort:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-roots = Roots:
field-backup-excluded-items = Backup exclusions:
field-redirects = Redirects:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
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
label-unscanned = Unscanned
# This refers to a local file on the computer
label-file = File
label-game = Game
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Original name
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
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
prefer-alias-display = Display alias instead of original name
explanation-for-exclude-store-screenshots = In backups, exclude store-specific screenshots
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

ludusavi = Ludusavi
language = 언어
game-name = 이름
total-games = 게임
file-size = 크기
file-location = 위치
overall = 전체
status = 상태
cli-unrecognized-games = 다음 게임에 대한 정보가 없습니다:
cli-unable-to-request-confirmation = 확인을 요청할 수 없습니다.
    .winpty-workaround = Git Bash와 같은 Bash 에뮬레이터를 사용 중이라면, winpty를 사용해보세요.
cli-backup-id-with-multiple-games = 복수 게임 복원 시 백업 ID를 지정할 수 없습니다.
cli-invalid-backup-id = 잘못된 백업 ID
badge-failed = 실패
badge-duplicates = 중복됨
badge-duplicated = 복사됨
badge-ignored = 무시됨
badge-redirected-from = 출처: { $path }
badge-redirecting-to = TO: { $path }
some-entries-failed = 일부 항목을 처리하지 못했습니다. { badge-failed } 를 참조하세요. 해당 파일에 접근할 수 있는지, 파일의 경로가 너무 길지 않은지 확인하세요.
cli-game-line-item-redirected = 다음에서 리다이렉트되었습니다: { $path }
cli-game-line-item-redirecting = 다음으로 디다이렉트: { $path }
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
button-customize = 사용자 정의
button-exit = 종료
button-comment = Comment
button-lock = 잠금
button-unlock = 잠금해제
# This opens a download page.
button-get-app = { $app } 받기
button-validate = 검증
button-override-manifest = Override manifest
button-extend-manifest = Extend manifest
button-sort = Sort
button-download = Download
button-upload = Upload
button-ignore = Ignore
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
unable-to-configure-cloud = 클라우드를 구성할 수 없습니다.
unable-to-synchronize-with-cloud = 클라우드와 동기화 할 수 없습니다.
cloud-synchronize-conflict = 로컬 백업과 클라우드 백업이 충돌합니다. 이 문제를 해결하려면 업로드 또는 다운로드를 수행하세요.
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
processed-size-subset = { $processed-size } 중 { $total-size }
field-backup-target = 여기에 백업
field-restore-source = 다음에서 복원
field-custom-files = 경로
field-custom-registry = 레지스트리
field-sort = 분류
field-redirect-source =
    .placeholder = 출처 (원래 위치)
field-redirect-target =
    .placeholder = 대상(새 위치)
field-roots = 경로
field-backup-excluded-items = 백업 제외:
field-redirects = 리디엑션
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
# The compression level determines how much compresison we perform.
field-backup-compression-level = 수준
label-manifest = Manifest
# This shows the time when we checked for an update to the manifest.
label-checked = Checked
# This shows the time when we found an update to the manifest.
label-updated = 업데이트됨
label-new = 신규
label-removed = 제거됨
label-comment = Comment
label-unchanged = 변경 안 됨
label-backup = Backup
label-scan = 스캔
label-filter = Filter
label-unique = Unique
label-complete = 완료됨
label-partial = Partial
label-enabled = 활성화됨
label-disabled = 비활성화됨
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = 스레드
label-cloud = 클라우드
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
label-host = 호스트
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = 포트
label-username = 아이디
label-password = 비밀번호
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = 공급자
label-custom = 사용자 정의
label-none = 없음
label-change-count = Changes: { $total }
label-unscanned = Unscanned
# This refers to a local file on the computer
label-file = 파일
label-game = 게임
# Aliases are alternative titles for the same game.
label-alias = 별칭
label-original-name = 원래 이름
# Which manifest a game's data came from
label-source = 출처
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
store-other = 기타
backup-format-simple = 간단
backup-format-zip = Zip
compression-none = 압축 없음
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = 테마
theme-light = 라이트
theme-dark = 다크
redirect-bidirectional = 양방향
reverse-redirects-when-restoring = Reverse sequence of redirects when restoring
show-disabled-games = Show disabled games
show-unchanged-games = 변경되지 않은 게임 보기
show-unscanned-games = 스캔 되지 않은 게임 보기
override-max-threads = Override max threads
synchronize-automatically = 자동 동기화
prefer-alias-display = 원래 이름 대신 별칭 표시
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = 백업에서 스토어별 스크린샷 제외하기
explanation-for-exclude-cloud-games = 클라우드를 지원하는 플랫폼 게임들은 백업 안 함
consider-doing-a-preview = 먼저 미리 보기를 수행하여 예상치 못한 일이 발생하지 않도록 하세요.
confirm-backup =
    백업을 진행하시겠습니까 { $path-action ->
        [merge] 새 저장 데이터가 대상 폴더에 병합됩니다:
       *[create] 대상 폴더가 생성됩니다:
    }
confirm-restore =
    복원을 진행하시겠습니까?
    현재 파일을 여기에서 백업한 파일로 덮어씁니다:
confirm-cloud-upload =
    클라우드 파일을 로컬 파일로 교체하시겠습니까?
    클라우드 파일 ({ $cloud-path }) 이 로컬 파일 ({ $local-path }) 의 정확한 복사본이 됩니다.
    클라우드의 파일은 필요에 따라 업데이트되거나 삭제됩니다.
confirm-cloud-download =
    로컬 파일을 클라우드 파일로 교체하시겠습니까?
    로컬 파일 ({ $local-path }) 이 클라우드 파일 ({ $cloud-path }) 의 정확한 복사본이 됩니다.
    로컬 파일은 필요에 따라 업데이트되거나 삭제됩니다.
confirm-add-missing-roots = 이 경로를 추가하시겠습니까?
no-missing-roots = 추가할 경로를 찾을 수 없습니다.
loading = 로드 중...
preparing-backup-target = 백업 파일 준비 중...
updating-manifest = 목록 업데이트 중...
no-cloud-changes = 동기화할 변경 사항 없음
backups-are-valid = 백업이 유효합니다.
backups-are-invalid =
    이 게임들의 백업이 유효하지 않은 것 같습니다.
    이러한 게임에 대한 새로운 전체 백업을 생성하시겠습니까?
saves-found = 세이브 데이터 찾음.
no-saves-found = 세이브 데이터를 찾을 수 없음.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = 다시 묻지 않음
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = 재시작 필요
prefix-error = 오류: { $message }
prefix-warning = 경고: { $message }
cloud-app-unavailable = { $app } 을 사용할 수 없으므로 클라우드 백업이 비활성화 됩니다.
cloud-not-configured = 클라우드 시스템이 구성되어 있지 않으므로 클라우드 백업이 비활성화됩니다.
cloud-path-invalid = 백업 경로가 잘못되어 클라우드 백업이 비활성화됩니다.
game-is-unrecognized = Ludusavi가 이 게임을 인식할 수 없습니다.
game-has-nothing-to-restore = 이 게임은 복원할 백업이 없습니다.
launch-game-after-error = 게임을 실행할까요?
game-did-not-launch = 게임 시작 실패.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = { $game } 의 세이브 데이터를 백업할까요?
    .failed = { $game } 의 세이브 데이터 백업 실패
restore-specific-game =
    .confirm = { $game } 의 세이브 데이터를 복원할까요?
    .failed = { $game } 의 세이브 데이터 복원 실패
new-version-check = 자동으로 애플리케이션 업데이트 확인
new-version-available = 업데이트를 할 수 있습니다: { $version }. 변경사항을 보시겠습니까?
custom-game-will-override = This custom game overrides a manifest entry
custom-game-will-extend = This custom game extends a manifest entry
operation-will-only-include-listed-games = This will only process the games that are currently listed

ludusavi = Ludusavi
language = 언어
font = 글꼴
game-name = 이름
total-games = 게임
file-size = 크기
file-location = 위치
overall = 전체
cli-backup-target-already-exists = 백업 대상이 이미 존재합니다 ( { $path } ). --path로 다른 위치를 지정하거나 --force로 강제로 삭제하세요.
cli-unrecognized-games = 다음 게임에 대한 정보가 없습니다:
cli-confirm-restoration = { $path } 에서 복원을 시작할까요?
cli-unable-to-request-confirmation = 확인을 요청할 수 없습니다.
    .winpty-workaround = Git Bash와 같은 Bash 에뮬레이터를 사용 중이라면, winpty를 사용해보세요.
cli-backup-id-with-multiple-games = Cannot specify backup ID when restoring multiple games.
cli-invalid-backup-id = Invalid backup ID.
badge-failed = 실패
badge-duplicates = 중복됨
badge-duplicated = 복사됨
badge-ignored = 무시됨
badge-redirected-from = 출처: { $path }
some-entries-failed = 일부 항목을 처리하지 못했습니다. { badge-failed } 를 참조하세요. 해당 파일에 접근할 수 있는지, 파일의 경로가 너무 길지 않은지 확인하세요.
cli-game-line-item-redirected = 다음에서 리다이렉트되었습니다: { $path }
button-backup = 백업
button-preview = 미리보기
button-restore = 복원
button-nav-backup = 백업 모드
button-nav-restore = 복원 모드
button-nav-custom-games = 사용자 지정 게임
button-nav-other = 기타
button-add-root = 최상위 디렉토리 추가
button-find-roots = 최상위 디렉토리 탐지
button-add-redirect = 리다이렉트 추가
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
toggle-backup-merge = Merge
field-restore-source = Restore from:
field-custom-files = Paths:
field-custom-registry = Registry:
field-search = Search:
field-sort = Sort:
field-redirect-source =
    .placeholder = Source (original location)
field-redirect-target =
    .placeholder = Target (new location)
field-backup-excluded-items = Backup exclusions:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Full:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic-config = Heroic Config
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home folder
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
theme = Theme
theme-light = Light
theme-dark = Dark
explanation-for-exclude-store-screenshots =
    In backups, exclude store-specific screenshots. Right now, this only applies
    to { store-steam } screenshots that you've taken. If a game has its own built-in
    screenshot functionality, this setting will not affect whether those
    screenshots are backed up.
consider-doing-a-preview =
    If you haven't already, consider doing a preview first so that there
    are no surprises.
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
updating-manifest = Updating manifest...

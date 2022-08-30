ludusavi = Ludusavi
language = اللغة
font = الخط
game-name = Name
total-games = Games
file-size = Size
file-location = Location
overall = Overall
cli-backup-target-already-exists = النسخ الإحتياطي موجود بالفعل ( { $path } ) إما أن تختار مسارا مختلفا أو تحذفه باستخدام --force.
cli-unrecognized-games = لا توجد معلومات عن هذه الألعاب:
cli-confirm-restoration = هل تريد الاستعادة من { $path }؟
cli-unable-to-request-confirmation = تعذر طلب التأكيد.
    .winpty-workaround = إذا كنت تستخدم محاكي Bash (مثل Git Bash)، فحاول تشغيل winpty.
badge-failed = فشل
badge-duplicates = النسخ المكررة
badge-duplicated = النسخ المكررة
badge-ignored = تجاهل
badge-redirected-from = من: { $path }
some-entries-failed = فشلت بعض الإدخالات في المعالجة؛ ابحث عن { badge-failed } في الإخراج للحصول على التفاصيل. تحقق مرة أخرى مما إذا كان يمكنك الوصول إلى هذه الملفات أو ما إذا كانت مساراتها طويلة جدا.
cli-game-line-item-redirected = أعيد توجيهه من: { $path }
button-backup = النسخ الاحتياطي
button-preview = معاينة
button-restore = استعادة
button-nav-backup = وضع النسخ الإحتياطي
button-nav-restore = وضع الإستعادة
button-nav-custom-games = العاب مخصصة
button-nav-other = اخرى
button-add-root = إضافة الجذر
button-find-roots = البحث عن الجذور
button-add-redirect = إضافة توجيه جديد
button-add-game = أضف لعبة
button-continue = متابعة
button-cancel = إلغاء
button-cancelling = جار الإلغاء...
button-okay = حسنا
button-select-all = تحديد الكل
button-deselect-all = إلغاء تحديد الكل
button-enable-all = تفعيل الكل
button-disable-all = تعطيل الكل
no-roots-are-configured = إضافة بعض الجذور لنسخ المزيد من البيانات احتياطياً.
config-is-invalid = خطأ: ملف التكوين غير صالح.
manifest-is-invalid = خطأ: ملف البيان غير صالح.
manifest-cannot-be-updated = خطأ: غير قادر على التحقق من وجود تحديث لملف البيان. هل اتصال الإنترنت الخاص بك منخفض؟
cannot-prepare-backup-target = خطأ: غير قادر على إعداد هدف النسخ الاحتياطي (إما إنشاء أو إفراغ المجلد). إذا كان لديك المجلد مفتوح في متصفح الملفات الخاص بك، حاول إغلاقه: { $path }
restoration-source-is-invalid = خطأ: مصدر الاستعادة غير صالح (إما غير موجود أو ليس دليل). الرجاء التحقق مرتين من الموقع: { $path }
registry-issue = خطأ: تم تخطي بعض إدخالات السجل.
unable-to-browse-file-system = Error: Unable to browse on your system.
unable-to-open-directory = Error: Unable to open directory:
unable-to-open-url = Error: Unable to open URL:
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
field-retention-full = Full:
field-retention-differential = Differential:
field-backup-format = Format:
field-backup-compression = Compression:
store-epic = Epic
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-gog-linux = GOG Linux
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Home folder
store-other-wine = Wine prefix
store-other = Other
sort-name = Name
sort-size = Size
sort-reversed = Reversed
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = None
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
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

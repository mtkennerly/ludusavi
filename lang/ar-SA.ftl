ludusavi = لودوسافي
language = اللغة
game-name = الاسم
total-games = الألعاب
file-size = الحجم
file-location = الموقع
overall = الكلي
status = Status
cli-unrecognized-games = لا توجد معلومات عن هذه الألعاب:
cli-unable-to-request-confirmation = تعذر طلب التأكيد.
    .winpty-workaround = إذا كنت تستخدم محاكي Bash (مثل Git Bash)، فحاول تشغيل winpty.
cli-backup-id-with-multiple-games = لا يمكن تحديد معرف النسخ الاحتياطي عند استعادة ألعاب متعددة.
cli-invalid-backup-id = معرف النسخ الاحتياطي غير صحيح.
badge-failed = فشل
badge-duplicates = النسخ المكررة
badge-duplicated = النسخ المكررة
badge-ignored = تجاهل
badge-redirected-from = من: { $path }
badge-redirecting-to = إلى: { $path }
some-entries-failed = فشلت بعض الإدخالات في المعالجة؛ ابحث عن { badge-failed } في الإخراج للحصول على التفاصيل. تحقق مرة أخرى مما إذا كان يمكنك الوصول إلى هذه الملفات أو ما إذا كانت مساراتها طويلة جدا.
cli-game-line-item-redirected = أعيد توجيهه من: { $path }
cli-game-line-item-redirecting = أعيد توجيهه إلى: { $path }
button-backup = النسخ الاحتياطي
button-preview = معاينة
button-restore = استعادة
button-nav-backup = وضع النسخ الإحتياطي
button-nav-restore = وضع الإستعادة
button-nav-custom-games = العاب مخصصة
button-nav-other = اخرى
button-add-game = أضف لعبة
button-continue = متابعة
button-cancel = إلغاء
button-cancelling = جار الإلغاء...
button-okay = حسنا
button-select-all = تحديد الكل
button-deselect-all = إلغاء تحديد الكل
button-enable-all = تفعيل الكل
button-disable-all = تعطيل الكل
button-customize = تخصيص
button-exit = خروج
button-comment = Comment
# This opens a download page.
button-get-app = Get { $app }
no-roots-are-configured = إضافة بعض الجذور لنسخ المزيد من البيانات احتياطياً.
config-is-invalid = خطأ: ملف التكوين غير صالح.
manifest-is-invalid = خطأ: ملف البيان غير صالح.
manifest-cannot-be-updated = خطأ: غير قادر على التحقق من وجود تحديث لملف البيان. هل اتصال الإنترنت الخاص بك منخفض؟
cannot-prepare-backup-target = خطأ: غير قادر على إعداد هدف النسخ الاحتياطي (إما إنشاء أو إفراغ المجلد). إذا كان لديك المجلد مفتوح في متصفح الملفات الخاص بك، حاول إغلاقه: { $path }
restoration-source-is-invalid = خطأ: مصدر الاستعادة غير صالح (إما غير موجود أو ليس دليل). الرجاء التحقق مرتين من الموقع: { $path }
registry-issue = خطأ: تم تخطي بعض إدخالات السجل.
unable-to-browse-file-system = خطأ: غير قادر لاستعراض على نظامك.
unable-to-open-directory = خطأ: تعذر فتح الدليل:
unable-to-open-url = خطأ: تعذر فتح الرابط:
unable-to-configure-cloud = Unable to configure cloud.
unable-to-synchronize-with-cloud = Unable to synchronize with cloud.
cloud-synchronize-conflict = Your local and cloud backups are in conflict. Perform an upload or download to resolve this.
command-unlaunched = Command did not launch: { $command }
command-terminated = Command terminated abruptly: { $command }
command-failed = Command failed with code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] اللعبة
       *[other] الألعاب
    }
processed-games-subset =
    { $processed-games } من { $total-games } { $total-games ->
        [one] لعبة
       *[other] الألعاب
    }
processed-size-subset = { $processed-size } من { $total-size }
field-backup-target = النسخ الاحتياطي إلى:
field-restore-source = استعادة من:
field-custom-files = ‮المسار:
field-custom-registry = السجل:
field-sort = فرز:
field-redirect-source =
    .placeholder = المصدر (الموقع الأصلي)
field-redirect-target =
    .placeholder = الهدف (موقع جديد)
field-roots = Roots:
field-backup-excluded-items = استثناء النسخ الاحتياطية:
field-redirects = إعادة توجيه:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = كامل:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = متغير:
field-backup-format = الصيغة:
field-backup-compression = ضغط:
# The compression level determines how much compresison we perform.
field-backup-compression-level = المستوى:
label-manifest = بيان
# This shows the time when we checked for an update to the manifest.
label-checked = متحقق
# This shows the time when we found an update to the manifest.
label-updated = محدث
label-new = جديد
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
store-epic = متجر Epic
store-gog = متجر GOG
store-gog-galaxy = متجر GOG Galaxy
store-heroic = Heroic
store-microsoft = متجر Microsoft
store-origin = متجر Origin
store-prime = متجر Prime
store-steam = متجر Steam
store-uplay = متجر Uplay
store-other-home = المجلد الرئيس
store-other-wine = Wine prefix
store-other = أخرى
backup-format-simple = بسيط
backup-format-zip = Zip
compression-none = لا شيء
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = السمة
theme-light = فاتح
theme-dark = مظلم
redirect-bidirectional = ثنائي الاتجاه
show-deselected-games = Show deselected games
show-unchanged-games = Show unchanged games
show-unscanned-games = Show unscanned games
override-max-threads = Override max threads
synchronize-automatically = Synchronize automatically
explanation-for-exclude-store-screenshots = في النسخ الاحتياطية، استبعاد لقطات الشاشة الخاصة بالمتجر
consider-doing-a-preview =
    إذا لم تكن قد قمت بالفعل ، فكر في إجراء معاينة أولاً حتى لا يكون هناك
    مفاجآت.
confirm-backup =
    هل أنت متأكد من أنك تريد المتابعة مع النسخ الاحتياطي؟ { $path-action ->
        [merge] سيتم دمج بيانات حفظ جديدة في المجلد المستهدف:
       *[create] سيتم إنشاء المجلد المستهدف:
    }
confirm-restore =
    هل أنت متأكد من أنك تريد المضي قدما في الاستعادة؟
    سيؤدي هذا إلى استبدال أي ملفات حالية مع النسخ الاحتياطية من هنا:
confirm-cloud-upload =
    Do you want to synchronize your local files to the cloud?
    Your cloud files ({ $cloud-path }) will become an exact copy of your local files ({ $local-path }).
    Files in the cloud will be updated or deleted as necessary.
confirm-cloud-download =
    Do you want to synchronize your cloud files to this system?
    Your local files ({ $local-path }) will become an exact copy of your cloud files ({ $cloud-path }).
    Local files will be updated or deleted as necessary.
confirm-add-missing-roots = إضافة هذه الجذور؟
no-missing-roots = لا توجد جذور إضافية.
loading = Loading...
preparing-backup-target = جارِ إعداد دليل النسخ الاحتياطي...
updating-manifest = تحديث البيان...
no-cloud-changes = No changes to synchronize
saves-found = العثور على بيانات محفوظة.
no-saves-found = لا توجد بيانات محفوظة.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = لا تأكيد
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = restart required
prefix-error = Error: { $message }
prefix-warning = Warning: { $message }
cloud-app-unavailable = Cloud backups are disabled because { $app } is not available.
cloud-not-configured = Cloud backups are disabled because no cloud system is configured.
cloud-path-invalid = Cloud backups are disabled because the backup path is invalid.

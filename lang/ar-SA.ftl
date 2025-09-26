ludusavi = لودوسافي
language = اللغة
game-name = الاسم
total-games = الألعاب
file-size = الحجم
file-location = الموقع
overall = الكلي
status = الحالة
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
button-comment = تعليق
button-lock = قفل
button-unlock = إلغاء القُفْل
# This opens a download page.
button-get-app = احصل على { $app }
button-validate = تحقق
button-override-manifest = تجاوز اللائحة
button-extend-manifest = توسيع اللائحة
button-sort = فرز
button-download = Download
button-upload = Upload
button-ignore = Ignore
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
unable-to-configure-cloud = تعذر تكوين السحابة.
unable-to-synchronize-with-cloud = تعذر المزامنة مع السحابة.
cloud-synchronize-conflict = النسخ الاحتياطي المحلي والسحابي مختلف. حمل أو نزل لحل هذه المشكلة.
command-unlaunched = الأمر لم يبدأ: { $command }
command-terminated = انهي الأمر فجأة: { $command }
command-failed = فشل الأمر مع الرمز { $code }: { $command }
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
field-roots = الجذور:
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
label-removed = أُزيل
label-comment = تعليق
label-unchanged = غير مغيّر
label-backup = النسخ الاحتياطي
label-scan = نسخ
label-filter = تصفية
label-unique = فريد
label-complete = مكتمل
label-partial = جزئي
label-enabled = مفعّل
label-disabled = معطَّل
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = المواضيع
label-cloud = سحابة
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = عن بُعد
label-remote-name = اسم عن بعد
label-folder = المجلد
# An executable file
label-executable = مِلَفّ تنفيذي
# Options given to a command line program
label-arguments = الاختيارات
label-url = الرابط
# https://en.wikipedia.org/wiki/Host_(network)
label-host = المضيف
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = المنفذ
label-username = اسم المستخدم
label-password = كلمة المرور
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = المزود
label-custom = مُخصّص
label-none = لا شيء
label-change-count = التغييرات: { $total }
label-unscanned = غير مفحوص
# This refers to a local file on the computer
label-file = الملف
label-game = اللعبة
# Aliases are alternative titles for the same game.
label-alias = الاسم المستعار
label-original-name = الاسم الأصلي
# Which manifest a game's data came from
label-source = المصدر
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = البيانات الرئيسية
# This refers to how we integrate a custom game with the manifest data.
label-integration = دمج
# This is a folder name where a specific game is installed
label-installed-name = Installed name
store-ea = EA
store-epic = متجر Epic
store-gog = متجر GOG
store-gog-galaxy = متجر GOG Galaxy
store-heroic = Heroic
store-legendary = Legendary
store-lutris = Lutris
store-microsoft = متجر Microsoft
store-origin = متجر Origin
store-prime = متجر Prime
store-steam = متجر Steam
store-uplay = متجر Uplay
store-other-home = المجلد الرئيس
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = مجلد Wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = مجلد Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = مجلد Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = مجلد Mac
store-other = أخرى
backup-format-simple = بسيط
backup-format-zip = ملف مضغوط بصيغة Zip
compression-none = لا شيء
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = السمة
theme-light = فاتح
theme-dark = مظلم
redirect-bidirectional = ثنائي الاتجاه
reverse-redirects-when-restoring = أعكس تسلسل إعادة التوجيه عند الاستعادة
show-disabled-games = إظهار الألعاب المعطلة
show-unchanged-games = إظهار الألعاب التي لم تتغير
show-unscanned-games = إظهار الألعاب الغير منسوخة
override-max-threads = تجاوز الحد الأقصى للموضوعات
synchronize-automatically = المزامنة تلقائياً
prefer-alias-display = عرض الاسم المستعار بدلاً من الاسم الأصلي
skip-unconstructive-backups = Skip backup when data would be removed, but not added or updated
explanation-for-exclude-store-screenshots = في النسخ الاحتياطية، استبعاد لقطات الشاشة الخاصة بالمتجر
explanation-for-exclude-cloud-games = لا تقم بعمل نسخة احتياطية للألعاب التي تدعم السحابة على هذه المنصات
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
    هل تريد استبدال ملفات السحابة الخاصة بك بملفاتك المحلية؟
    ملفات السحابة الخاصة بك ({ $cloud-path }) ستكون نسخة مطابقة من ملفاتك المحلية ({ $local-path }).
    سيتم تحديث الملفات الموجودة في السحابة أو حذفها حسب الضرورة.
confirm-cloud-download =
    هل تريد استبدال ملفاتك المحلية بملفاتك السحابية؟
    الملفات المحلية الخاصة بك ({ $local-path }) ستكون نسخة مطابقة من الملفات السحابية الخاصة بك ({ $cloud-path }).
    سيتم تحديث الملفات المحلية أو حذفها حسب الضرورة.
confirm-add-missing-roots = إضافة هذه الجذور؟
no-missing-roots = لا توجد جذور إضافية.
loading = تحميل...
preparing-backup-target = جارِ إعداد دليل النسخ الاحتياطي...
updating-manifest = تحديث البيان...
no-cloud-changes = لا توجد تغييرات للمزامنة
backups-are-valid = النسخ الاحتياطي الخاص بك صحيح.
backups-are-invalid =
    يبدو بأن النُسخ الاحتياطية لهذه الألعاب غير صالحة.
    هل تريد إنشاء نُسخ احتياطية كاملة جديدة لهذه الألعاب؟
saves-found = العثور على بيانات محفوظة.
no-saves-found = لا توجد بيانات محفوظة.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = لا تأكيد
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = مطلوب إعادة التشغيل
prefix-error = خطأ: { $message }
prefix-warning = تحذير: { $message }
cloud-app-unavailable = النسخ الاحتياطي السحابي معطل لأن { $app } غير متوفر.
cloud-not-configured = النسخ الاحتياطي السحابي معطل لأنه لم يُنشأ أي نظام سحابي.
cloud-path-invalid = النسخ الاحتياطي السحابي معطل لأن مسار النسخ الاحتياطي غير صالح.
game-is-unrecognized = لودوسافي لا يتعرف على هذه اللعبة.
game-has-nothing-to-restore = هذه اللعبة ليست لديها نسخة احتياطية لاستعادتها.
launch-game-after-error = هل ترغب في تشغيل اللعبة على أي حال؟
game-did-not-launch = فشل تشغيل اللعبة.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = هل ترغب في عمل نسخة احتياطية لبيانات الحفظ ل { $game }؟
    .failed = فشل في عمل نسخة احتياطية لبيانات الحفظ ل { $game }
restore-specific-game =
    .confirm = هل تريد استعادة بيانات الحفظ ل { $game }؟
    .failed = فشل في استعادة بيانات الحفظ ل { $game }
new-version-check = التحقق من وجود تحديثات التطبيق تلقائيا
new-version-available = يتوفر تحديث للتطبيق: { $version }. هل ترغب في عرض ملاحظات الإصدار؟
custom-game-will-override = هذه اللعبة المخصصة تتجاوز عنصر في اللائحة
custom-game-will-extend = هذه اللعبة المخصصة توسع عنصر في اللائحة
operation-will-only-include-listed-games = سيؤدي هذا فقط إلى معالجة الألعاب المدرجة حاليا

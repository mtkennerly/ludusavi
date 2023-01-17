ludusavi = لودوسافي
language = اللغة
font = الخط
game-name = الاسم
total-games = الألعاب
file-size = الحجم
file-location = الموقع
overall = الكلي
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
cli-game-line-item-redirecting = Redirecting to: { $path }
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
button-customize = تخصيص
button-exit = خروج
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
toggle-backup-merge = دمج
field-restore-source = استعادة من:
field-custom-files = ‮المسار:
field-custom-registry = السجل:
field-search = بحث:
field-sort = فرز:
field-redirect-source =
    .placeholder = المصدر (الموقع الأصلي)
field-redirect-target =
    .placeholder = الهدف (موقع جديد)
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
sort-reversed = معكوس
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
explanation-for-exclude-store-screenshots =
    في النسخ الاحتياطية، استبعاد لقطات الشاشة الخاصة بالمتجر. في الوقت الحالي، ينطبق هذا فقط
    على { store-steam }. إذا كانت اللعبة تحتوي على مِيزة لقطة الشاشة الخاصة بها، فإن هذا الإعداد لن يؤثر على ما إذا تم نسخ لقطات الشاشة.
consider-doing-a-preview =
    إذا لم تكن قد قمت بالفعل ، فكر في إجراء معاينة أولاً حتى لا يكون هناك
    مفاجآت.
confirm-backup =
    هل أنت متأكد من أنك تريد المتابعة مع النسخ الاحتياطي؟ { $path-action ->
        [merge] سيتم دمج بيانات حفظ جديدة في المجلد المستهدف:
        [recreate] سيتم حذف المجلد المستهدف وإعادة إنشاؤه من الصفر:
       *[create] سيتم إنشاء المجلد المستهدف:
    }
confirm-restore =
    هل أنت متأكد من أنك تريد المضي قدما في الاستعادة؟
    سيؤدي هذا إلى استبدال أي ملفات حالية مع النسخ الاحتياطية من هنا:
confirm-add-missing-roots = إضافة هذه الجذور؟
no-missing-roots = لا توجد جذور إضافية.
preparing-backup-target = جارِ إعداد دليل النسخ الاحتياطي...
updating-manifest = تحديث البيان...
saves-found = العثور على بيانات محفوظة.
no-saves-found = لا توجد بيانات محفوظة.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = لا تأكيد

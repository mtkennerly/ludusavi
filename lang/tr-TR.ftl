ludusavi = Ludusavi
language = Dil
game-name = Oyun İsmi
total-games = Oyunlar
file-size = Boyut
file-location = Konum
overall = Genel
status = Durum
cli-unrecognized-games = Bu oyunlar hakkında bilgi bulunamadı:
cli-unable-to-request-confirmation = Unable to request confirmation.
    .winpty-workaround = Eğer bash emulator kullanıyorsan (Git Bash gibi), winpty'i çalıştırmayı dene.
cli-backup-id-with-multiple-games = Oyunların yedeği geri yüklenirken backup ID belirlenemiyor.
cli-invalid-backup-id = Geçersiz backup ID.
badge-failed = BAŞARISIZ
badge-duplicates = KOPYALAR
badge-duplicated = KOPYALANMIŞ
badge-ignored = IGNORED
badge-redirected-from = Buradan: { $path }
badge-redirecting-to = Şuraya: { $path }
some-entries-failed = Bazı girdilerin işlenmesi başarısız; detay için çıktı da { badge-failed } seçeneğine bak. O dosyalara erişim var mı veya dosya yolları çok uzun mu diye tekrar kontrol et.
cli-game-line-item-redirected = Şuradan yönlendirildi: { $path }
cli-game-line-item-redirecting = Şuraya yönlendiriliyor: { $path }
button-backup = Yedekle
button-preview = Önizle
button-restore = Geri Yükle
button-nav-backup = YEDEKLEME MODU
button-nav-restore = GERİ YÜKLEME MODU
button-nav-custom-games = ÖZEL OYUNLAR
button-nav-other = DİĞER
button-add-game = Oyun ekle
button-continue = Devam Et
button-cancel = İptal
button-cancelling = İptal ediliyor...
button-okay = Tamam
button-select-all = Tümünü seç
button-deselect-all = Tüm seçimi kaldır
button-enable-all = Hepsini etkinleştir
button-disable-all = Hepsini devre dışı bırak
button-customize = Kişiselleştir
button-exit = Çıkış
button-comment = Yorum Yap
button-lock = Kilitle
button-unlock = Kilidi aç
# This opens a download page.
button-get-app = İndir { $app }
button-validate = Doğrula
button-override-manifest = Bildiriyi geçersiz kıl
button-extend-manifest = Bildiriyi genişlet
button-sort = Filtrele
button-download = İndir
button-upload = Yükle
button-ignore = Göz ardı et
no-roots-are-configured = Daha fazla veri yedeklemek için daha fazla kök dizin ekleyin.
config-is-invalid = Hata: Seçenekler dosyası geçersiz.
manifest-is-invalid = Hata: Bildiri dosyası geçersiz.
manifest-cannot-be-updated = Hata: Bildiri dosyasında güncelleme olup olmadığı kontrol edilemiyor. İnternet bağlantınız mı koptu?
cannot-prepare-backup-target = Hata: Yedekleme hazırlanamıyor (klasör oluşturulurken veya boşaltılırken). Dosya gezgininde klasör açıksa kapatmayı deneyin: { $path }
restoration-source-is-invalid = Hata: Geri yükleme kaynağı geçersiz (ya mevcut değil ya da bir dizin değil). Lütfen konumu tekrar kontrol edin: { $path }
registry-issue = Hata: Bazı kayıt defteri girdileri atlandı.
unable-to-browse-file-system = Hata: Sisteminizde göz atılamıyor.
unable-to-open-directory = Hata: Dizin açılamıyor:
unable-to-open-url = Hata: URL açılamıyor:
unable-to-configure-cloud = Bulut yapılandırılamıyor.
unable-to-synchronize-with-cloud = Bulut eşitlemesi yapılamıyor.
cloud-synchronize-conflict = Yerel ve bulut yedeklemeleriniz çakışıyor. Bir yükleme ya da indirme yaparak çözüm sağlayın.
command-unlaunched = Komut yürütülemedi: { $command }
command-terminated = Komut ani şekilde sonlandı: { $command }
command-failed = Komut şu kodla başarısız oldu { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [bir] oyun
       *[diger] oyunlar
    }
processed-games-subset =
    { $total-games } { $total-games} içinden { $processed-games   ->
        [bir] oyun
       *[diger] oyunlar
    }
processed-size-subset = { $total-size } içinden { $processed-size }
field-backup-target = Şuraya yedekle:
field-restore-source = Şuradan geri yükle:
field-custom-files = Yollar:
field-custom-registry = Kayıt Defteri:
field-sort = Sırala:
field-redirect-source =
    .placeholder = Kaynak (orjinal yer)
field-redirect-target =
    .placeholder = Hedef (yeni yer)
field-roots = Kök dizinler:
field-backup-excluded-items = Yedekleme istisnaları:
field-redirects = Yönlendirmeler:
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Tam:
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Değişiklikler:
field-backup-format = Biçim:
field-backup-compression = Sıkıştırma:
# The compression level determines how much compresison we perform.
field-backup-compression-level = Seviye:
label-manifest = Bildiri
# This shows the time when we checked for an update to the manifest.
label-checked = Kontrol edildi
# This shows the time when we found an update to the manifest.
label-updated = Güncellendi
label-new = Yeni
label-removed = Kaldırıldı
label-comment = Yorum
label-unchanged = Değişmedi
label-backup = Yedekleme
label-scan = Tara
label-filter = Filtre
label-unique = Benzersiz
label-complete = Tamamla
label-partial = Kısmen
label-enabled = Etkin
label-disabled = Devre dışı
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = İş Parçacıkları
label-cloud = Bulut
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Bulut yedekleme
label-remote-name = Bulut yedekleme ismi
label-folder = Klasör
# An executable file
label-executable = Yürütülebilir
# Options given to a command line program
label-arguments = Değişkenler
label-url = Bağlantı
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Sunucu
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Bağlantı Noktası
label-username = Kullanıcı Adı
label-password = Şifre
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Sağlayıcı
label-custom = Özel
label-none = Hiçbiri
label-change-count = Değişiklikler: { $total }
label-unscanned = Taranmamış
# This refers to a local file on the computer
label-file = Dosya
label-game = Oyun
# Aliases are alternative titles for the same game.
label-alias = Takma Ad
label-original-name = Orjinal ad
# Which manifest a game's data came from
label-source = Kaynak
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Birincil bildirim
# This refers to how we integrate a custom game with the manifest data.
label-integration = Entegrasyon
# This is a folder name where a specific game is installed
label-installed-name = Yüklenmiş adı
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
store-other-home = Ana klasör
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Wine sürücüsü
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Windows sürücüsü
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Linux sürücüsü
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Mac sürücüsü
store-other = Diğer
backup-format-simple = Basit
backup-format-zip = Zip
compression-none = Yok
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Tema
theme-light = Beyaz
theme-dark = Siyah
redirect-bidirectional = Çift yönlü
reverse-redirects-when-restoring = Geri yüklerken yönlendirme sırasını tersine çevir
show-disabled-games = Engellenmiş oyunları göster
show-unchanged-games = Değişmeyen oyunları göster
show-unscanned-games = Taranmamış oyunları göster
override-max-threads = Maksimum iş parçacığını geçersiz kıl
synchronize-automatically = Otomatik olarak senkronize et
prefer-alias-display = Orijinal ad yerine takma adı görüntüle
skip-unconstructive-backups = Verilerin kaldırılacağı ancak eklenmeyeceği veya güncellenmeyeceği durumlarda yedeklemeyi atla
explanation-for-exclude-store-screenshots = Yedeklemelerde mağazaya özel ekran görüntülerini hariç tutun
explanation-for-exclude-cloud-games = Bu platformlarda bulut destekli oyunları yedekleme
consider-doing-a-preview =
    Henüz yapmadıysan, önce bir ön izleme yapmayı düşün, böylece
    sürprizlerle karşılaşmayacaksın.
confirm-backup =
    Yedeklemeye devam etmek istediğinizden emin misiniz? { $path-action ->
        [merge] Yeni kaydetme verileri hedef klasörle birleştirilecek:
       *[create] Hedef klasör oluşturulur:
    }
confirm-restore =
    Restorasyona devam etmek istediğinizden emin misiniz?
    Bu, buradaki yedekleri içeren mevcut dosyaların üzerine yazacaktır:
confirm-cloud-upload =
    Bulut dosyalarınızı yerel dosyalarınızla değiştirmek ister misiniz?
    Bulut dosyalarınız ({ $cloud-path }) yerel dosyalarınızın ({ $local-path }) tam bir kopyası haline gelecektir.
    Buluttaki dosyalar gerektiği şekilde güncellenecek veya silinecektir.
confirm-cloud-download =
    Yerel dosyalarınızı bulut dosyalarınızla değiştirmek ister misiniz?
    Yerel dosyalarınız ({ $local-path }), bulut dosyalarınızın ({ $cloud-path }) tam bir kopyası haline gelecektir.
    Yerel dosyalar gerektiği şekilde güncellenecek veya silinecektir.
confirm-add-missing-roots = Bu kök dizinler eklensin mi?
no-missing-roots = Başka kök dizin bulunamadı.
loading = Yükleniyor...
preparing-backup-target = Yedekleme dizini hazırlanıyor...
updating-manifest = Bildiri güncelleniyor...
no-cloud-changes = Senkronize edilecek değişiklik yok
backups-are-valid = Yedeklemeleriniz geçerlidir.
backups-are-invalid =
    Bu oyunların yedeklemeleri geçersiz görünüyor.
    Bu oyunlar için yeni tam yedeklemeler oluşturmak istiyor musunuz?
saves-found = Kayıtlı veri mevcut.
no-saves-found = Kayıtlı veri bulunamadı.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = doğrulamasız
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = yeniden başlatma gerekli
prefix-error = Hata: { $message }
prefix-warning = Uyarı: { $message }
cloud-app-unavailable = { $app } kullanılamadığından bulut yedeklemeleri devre dışı bırakıldı.
cloud-not-configured = Hiçbir bulut sistemi yapılandırılmadığından bulut yedeklemeleri devre dışı bırakıldı.
cloud-path-invalid = Yedekleme yolu geçersiz olduğundan bulut yedeklemeleri devre dışı bırakıldı.
game-is-unrecognized = Ludusavi bu oyunu tanımıyor.
game-has-nothing-to-restore = Bu oyunun geri yüklenecek bir yedeği yok.
launch-game-after-error = Yine de oyun başlatılsın mı?
game-did-not-launch = Oyun başlatılamadı.
backup-is-newer-than-current-data = Var olan yedekleme, güncel veriden daha yeni.
backup-is-older-than-current-data = Var olan yedekleme, güncel veriden daha eski.
back-up-specific-game =
    .confirm = { $game } için kayıt verileri yedeklensin mi?
    .failed = { $game } için kayıt verileri yedeklenemedi
restore-specific-game =
    .confirm = { $game } için kayıt verileri geri yüklensin mi?
    .failed = { $game } için kayıt verileri geri yüklenemedi
new-version-check = Güncellemelerini otomatik olarak kontrol et
new-version-available = Güncelleme mevcut: { $version }. Sürüm notlarını görüntülemek ister misiniz?
custom-game-will-override = Bu özel oyun, bildirim girişini geçersiz kılıyor
custom-game-will-extend = Bu özel oyun, manifest girişini genişletiyor
operation-will-only-include-listed-games = Bu yalnızca şu anda listelenen oyunları işleyecektir

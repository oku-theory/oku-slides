# Ajan: yüzeyler, onaylar, ileti dizisi.

agent-surface-cancel = İptal
agent-surface-confirm = Onayla
agent-surface-confirm-danger = Evet, yap
agent-surface-heard = Duyulan: “{ $text }”
agent-surface-say-or-click = Cevabınızı söyleyin veya bir düğmeye tıklayın.
agent-surface-said-too-early = Çok erken söylendi: pencereyi bekleyin, sonra tekrar söyleyin.
agent-surface-click-to-confirm = Onaylamak için düğmeye tıklayın.
agent-surface-verb-replace = Değiştir
agent-surface-verb-merge = Birleştir
agent-surface-verb-discard = Çöpe at
agent-surface-verb-delete = Sil
agent-surface-verb-use-api = Api kullan
agent-surface-verb-install = Yükle
agent-surface-verb-quit = Uygulamadan çık
agent-surface-verb-close = Kapat
agent-surface-verb-import = İçe aktar
agent-surface-rewrite = Yeniden ifade etme
agent-surface-pick = { $n } seç
agent-surface-keep = Orijinali koru
agent-surface-edit = Uygulamadan önce düzenle
agent-surface-close = Kapat
agent-surface-themes = Temalar
agent-surface-variants = Varyantlar
agent-surface-permission-always = her zaman için
agent-surface-permission-title = Ajan bir izin istiyor
agent-surface-permission-guides = Bu tur, { $names } için alınan kılavuzu içerir.
agent-undo-turn = Ajan: { $text }

agent-tools-unavailable = Ajanın araçları başlatılamadı.

agent-undo-apply = Ajan değişikliği
agent-undo-variant = Varyant: { $label }
agent-undo-param = Ayar: { $label }
agent-undo-rewrite = Yeniden ifade etme

agent-confirm-save = Sunum { $path } olarak kaydedilsin mi?
agent-confirm-save-over = { $path } değiştirilsin mi? Mevcut dosyanın üzerine yazılacak.
agent-confirm-open = { $path } açılsın mı?
agent-open-elsewhere = Bu dosya zaten başka bir sekmede açık.
agent-confirm-new = Yeni bir sunum başlatılsın mı?
agent-confirm-new-dirty = Yeni bir sunum başlatılsın mı? Kaydedilmemiş değişiklikler kaybolacak.
agent-confirm-copy-pack = “{ $name }” paketinin bir kopyası kaydedilsin mi? Paketin kendisi olduğu gibi kalır.
agent-confirm-copy-edited-theme = “{ $name }” temasının bir kopyası kaydedilsin mi? Temanın kendisi olduğu gibi kalır.
agent-confirm-delete-pack = “{ $name }” paketi silinsin mi? Bu geri alınamaz.
agent-confirm-edit-theme = “{ $name }” temasının yeni bir sürümü kaydedilsin mi?
agent-confirm-copy-theme = Yerleşik “{ $name }” temasının bir varyantı oluşturulsun mu? Yerleşik tema değişmeden kalır.
agent-confirm-setting = { $key } değiştirilsin mi: { $old } → { $new }?
agent-confirm-move = Sunum { $path } konumuna taşınsın mı? Eski dosya silinecek.
agent-confirm-merge = { $path } bu sunumla birleştirilsin mi? Bu işlem geri alınamaz.
agent-confirm-save-copy = Sunumun bir kopyası { $path } olarak kaydedilsin mi?
agent-confirm-dictation-api = Dikte ettiğiniz metin { $host } ({ $url }) adresine, { $model } modeliyle ve { $var } içindeki sırla gönderilsin mi?
agent-confirm-dictation-download = { $name } kullanılsın ve şimdi indirilsin mi ({ $size })?
agent-confirm-dictation-language = Apple dikte kullanılsın ve dili şimdi indirilsin mi?
agent-confirm-agent = Hiçbir yanıt çalışmadığında uygulama ajanı { $name } ({ $command }) olarak değiştirilsin mi?
agent-confirm-agent-builtin = Hiçbir yanıt çalışmadığında uygulama ajanı { $name } olarak değiştirilsin mi?
agent-confirm-agent-model = Yerel ajan modeli indirilsin mi ({ $size })?

agent-not-found = Ajan bulunamadı: { $command } ne o yolda ne de uygulamanın PATH'inde var. Tam yolunu [agent.presets] içinde belirtin.
agent-spawn-failed = Ajan başlatılamadı: { $command }. Ayar [agent.presets].
agent-no-http = Bu ajan Oku Slides'ın araçlarına bağlanamıyor.
agent-local-no-program = Yerel ajan yalnızca Oku Slides uygulamasının içinde çalışır.
agent-local-needs-download = Yerel ajanın önce modelini ({ $gigabytes } GB) indirmesi gerekiyor.
agent-local-intel-mac = Yerel ajan, Apple silikonlu bir Mac gerektirir.
agent-local-unsupported-os = Yerel ajan bu sistemde henüz kullanılamıyor.
agent-local-apple-off = Yerel ajan Apple Intelligence gerektirir: Sistem Ayarları'ndan açın.
agent-local-apple-not-ready = Apple'ın modeli hâlâ hazırlanıyor: yerel ajanı birkaç dakika sonra yeniden deneyin.
agent-local-apple-unsupported = Apple'ın modeli, Apple Intelligence uyumlu ve macOS 26 veya üstü bir Mac gerektirir.
agent-model-download = Modeli indir
agent-model-downloading = Model indiriliyor… %{ $percent }
agent-model-cancel = İptal
agent-model-download-failed = Model indirilemedi.
agent-model-download-integrity = İndirilen model bozuktu ve silindi.
agent-model-download-incomplete = Eksik indirme: { $expected } baytın { $got } baytı alındı.
agent-model-download-cancelled = İndirme iptal edildi.
agent-fallback-notice = Yerel ajan — { $reason }
agent-fallback-download = { $reason } Yerel ajan, modeli ({ $model }, { $gigabytes } GB) indirildiğinde devralabilir. Şimdi indirilsin mi?
agent-fallback-unusable = Hiçbir ajan çalışamıyor. Ana ajan: { $main } Yerel ajan: { $local }
local-agent-no-route = Yerel ajan ne yapacağını anlamadı. Başka bir şekilde söylemeyi deneyin.
local-agent-tool-limit = Yerel ajan { $n } adımdan sonra bitirmeden durdu.
local-agent-invalid-call = Yerel ajan araçları doğru kullanamadı.
local-agent-engine-failed = Yerel ajanın modeli başarısız oldu.
local-agent-retry = Yeniden deneyin ya da ajan panelinden başka bir ajan seçin.
agent-stopped = Ajan durdu.
agent-failed = Ajan isteği reddetti: { $message }
agent-error-work-dir = Bu sekme için ajanın özel çalışma klasörü hazırlanamadı.
agent-tool-failed = Araç beklenmedik biçimde başarısız oldu.

agent-turn-tools = { $tools } araç
agent-turn-notes = { $notes } not
agent-turn-secs = { $secs } sn
agent-turn-stopped = Durduruldu.
agent-turn-interrupted = Kesildi.
agent-memory-lost = Ajan bu konuşmayı artık hatırlamıyor: baştan başlıyor.
agent-turn-max-tokens = Ajan uzunluk sınırına ulaştı.
agent-turn-max-turns = Ajan adım sınırına ulaştı.
agent-turn-refusal = Ajan reddetti.
agent-turn-ended = Tur bitti ({ $stop }).
agent-choice-cancelled = Soru iptal edildi
agent-choice-answered = ✓ { $answer }
agent-report-why = Neden
agent-trace = { $name }: { $count ->
    [one] { $count } değişiklik
   *[other] { $count } değişiklik
}
agent-trace-data = { $name }: veriler değişti
agent-trace-removed = { $name }: artık yok
agent-trace-more = { $count ->
    [one] ve { $count } diğeri
   *[other] ve { $count } diğeri
}
agent-trace-title = Değiştirilen öğeler

agent-error-file = Dosya okunamadı veya yazılamadı.
agent-error-unexpected-action = Beklenmeyen eylem: { $action }.
agent-error-unknown-surface = Bu kart artık açık değil.
agent-error-expired = Geçmiş bir turun eylemlerinin süresi doldu.
agent-error-incomplete = Bu kartın eylemi eksik.
agent-error-no-field = Bu ayar artık yok.
agent-error-no-variant = Bu varyant artık yok.
agent-error-arguments = Aracın argümanları geçersiz ({ $detail }): şemasına göre kontrol edin.
agent-error-unknown-tool = Bilinmeyen araç: { $name }.
agent-error-pack-id = Bu bir paketin kimliği değil: paketleri ve kimliklerini listelemek için packs aracını çağırın.
agent-error-asset-unknown = Bu görsel ne sunumda ne de yapıştırılan görseller arasında: yapıştırılmış bir görselin kimliğini kullanın.
agent-error-denied = Kullanıcı reddetti.
agent-error-no-turn = Devam eden bir tur yok: { $tool } yalnızca bir tur sırasında gösterilir.
agent-error-tab-no-turn = Bir sekmeyi yalnızca kullanıcıya yanıt verirken açın, gösterin ya da taşıyın.
agent-error-tab-limit = Bu turda zaten bir sekme açtınız, gösterdiniz ya da taşıdınız; orada bitirin ya da bir başkasından önce kullanıcıya sorun.
agent-error-continue-in-other-tab = Bu istek açtığınız veya gösterdiğiniz sekmede sürüyor: kullanıcıdan orada devam etmesini isteyin; bu sekme şimdilik hiçbir widget'ı değiştirmez.
agent-error-pack-open-in-tab = Bu paket başka bir sekmede düzenleniyor: kullanıcıdan önce orada bitirmesini isteyin.
agent-pack-opened-in-tab = { $shown ->
   [yes] Paket { $tab } sekmesinde açıldı ve şimdi gösteriliyor.
  *[no] Paket { $tab } sekmesinde arka planda açıldı: kullanıcının penceresi başka bir sekme gösteriyor.
} { $draft ->
   [set] İstek o sekmenin taslağında bekliyor: kullanıcıya orada gözden geçirip sonra göndermesini söyleyin.
   [kept] O sekmenin taslağında zaten bir şey vardı, olduğu gibi bırakıldı. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
   [too_long] İstek o sekmenin taslağı için fazla uzundu. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
   [unavailable] İstek o sekmenin taslağına ulaştırılamadı. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
  *[none] Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
} Bu sekme olduğu gibi kalır.
agent-theme-opened-in-tab = { $shown ->
   [yes] Tema { $tab } sekmesinde açıldı ve şimdi gösteriliyor.
  *[no] Tema { $tab } sekmesinde arka planda açıldı: kullanıcının penceresi başka bir sekme gösteriyor.
} { $draft ->
   [set] İstek o sekmenin taslağında bekliyor: kullanıcıya orada gözden geçirip sonra göndermesini söyleyin.
   [kept] O sekmenin taslağında zaten bir şey vardı, olduğu gibi bırakıldı. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
   [too_long] İstek o sekmenin taslağı için fazla uzundu. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
   [unavailable] İstek o sekmenin taslağına ulaştırılamadı. Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
  *[none] Kullanıcıya orada devam etmesini söyleyin: o sekmenin ajanı bu isteği bilmiyor.
} Bu sekme olduğu gibi kalır.
agent-error-turn-over = Bu çağrının turu bitti: hiçbir şey gösterilmedi.
agent-error-no-box-turn = Bu kutunun bağlanacağı bir tur yok: hiçbir şey gösterilmedi.
agent-error-no-file = Sunumun henüz bir dosyası yok: mutlak bir yol verin.
agent-error-path-relative = Yol mutlak olmalıdır.
agent-error-path-parent = Yol “..” içermemelidir.
agent-error-setting-locked = { $key } ayarı aracı tarafından değiştirilemez.
agent-error-setting = { $key } ayarı değiştirilemedi: değerini kontrol edin.
agent-error-history-index = Geçmişte gezinmek için bir sürümün sırası gerekir.
agent-error-no-pack = Bu sunum hiçbir paketi sabitlemiyor: pack verin (bkz. packs).
agent-error-history-not-opened = Sekme geçmişi zamanında açmadı: yeniden isteyin.
agent-error-locale = Bilinmeyen dil kodu. Sunulan bir dilin kodunu ya da system verin.
agent-error-empty-interface = Değiştirilecek bir şey yok: scheme, locale, guides, folded, pile, notes ya da code verin.
agent-error-layout-range = { $field } { $min } ile { $max } px arasında olmalı.
agent-error-layout-min = { $field } en az { $min } px olmalı.
agent-error-empty-dictation = Değiştirilecek bir şey yok: dikte ayarlarından en az birini verin.
agent-error-empty-speech = Değiştirilecek bir şey yok: en az bir sesli okuma ayarı verin.
agent-error-not-shown = Bu sekme penceresinde gösterilen sekme değil: kullanıcı onu gösterdiğinde yeniden isteyin.
agent-error-not-presenting = Deck burada sunulmuyor: önce sunumu start eylemiyle başlatın.
agent-error-present-slide = Bir slaytı göstermek için bu deck'teki bir slaydın id'si gerekir (bkz. deck_outline).
agent-error-undo-open-turn = undo_turn süren turu geri almaz: bu turun kendi değişiklikleri için undo kullanın.
agent-error-app-unavailable = Bunu yalnızca Oku Slides uygulaması yapabilir; burada kullanılamaz.
agent-error-app-closing = Oku Slides zaten kapanıyor.
agent-error-app-no-offer = Burada ertelenecek ya da yüklenecek bir güncelleme yok: önce güncellemeleri denetle.
agent-error-app-failed = Oku Slides güncellemesi başarısız oldu: bağlantıyı kontrol edin veya güncelleme bandını kullanın.
agent-error-after-turn-taken = Bu yanıt zaten uygulamadan çıkarak, bir güncelleme yükleyerek ya da kendi sekmesini kapatarak bitiyor.
agent-error-no-pack-update = Paket paneli bu paket için böyle bir güncelleme sunmuyor: show_packs'e bak.
agent-confirm-update-install = Oku Slides { $version } yüklensin mi? Uygulama bu yanıt bitince yeniden başlar.
agent-after-turn-quit-unsaved = Oku Slides kapanmadı: istediğinden beri bir sekmede kaydedilmemiş değişiklikler var.
agent-error-workspace-unavailable = Sekmeleri ve pencereleri yalnızca Oku Slides uygulaması değiştirebilir; burada kullanılamıyor.
agent-error-workspace-unknown-tab = Bu pencerede böyle bir sekme ya da pencere yok.
agent-error-workspace-unknown-window = Böyle bir pencere yok.
agent-error-workspace-index = Bu konum sekme çubuğunun sonunu aşıyor.
agent-error-workspace-closing = Oku Slides kapanıyor: sekmeler ve pencereler artık değişmiyor.
agent-confirm-close-tab = “{ $tab }” sekmesi kapatılsın mı?
agent-confirm-discard-tab = “{ $tab }” sekmesi kapatılsın mı? Kaydedilmemiş değişiklikleri kaybolacak.
agent-confirm-close-window = Bu pencere ve sekmeleri kapatılsın mı?
agent-confirm-discard-window = Bu pencere kapatılsın mı? { $tabs } içindeki kaydedilmemiş değişiklikler kaybolacak.
agent-error-window-last = Tek pencere bu: bunun yerine sekmelerini kapat ya da Oku Slides'tan çık.
agent-error-tab-index = Sekmenin nereye konacağını söyle: yeniden sıralamak için bir konum gerekir.
agent-error-tab-unused = Yalnızca yeniden sıralama ve taşıma bir konum alır, yalnızca taşıma bir pencere alır.
agent-after-turn-close-unsaved = Sekme kapatılmadı: istediğinden beri değişti.
agent-after-turn-close-not-shown = Sekme kapatılmadı: yanıt bittiğinde penceresinde başka bir sekme gösteriliyordu.
agent-confirm-quit = Oku Slides'tan çıkılsın mı? Uygulama bu yanıt bitince kapanır.
agent-error-quit-unsaved = Kaydedilmemiş iş içeren sekmeler: { $count }. Oku Slides'tan çıkılmadı: kullanıcı önce bunları kaydeder ya da kapatır.
agent-error-tune-unavailable = İnce ayar başlayamadı: ajan kullanılamıyor.
agent-error-tune-not-started = İnce ayar başlamadı: o sekmenin ajanı şimdilik kullanılamıyor; sekme nedenini söylüyor.
agent-confirm-import = { $path } yeni bir sekmede içe aktarılsın mı?
agent-import-opened-in-tab = { $shown ->
   [yes] Sunum { $tab } sekmesinde açıldı, şimdi gösteriliyor.
  *[no] Sunum { $tab } sekmesinde arka planda açıldı: kullanıcının penceresi başka bir sekme gösteriyor.
} { $tune ->
   [started] İnce ayarı orada, o sekmenin kendi konuşmasında başlıyor.
   [unavailable] İnce ayarı başlayamadı: kullanıcıya bunu o sekmede istemesini söyleyin.
  *[no] Kullanıcıya orada devam etmesini söyleyin.
} Bu sekme olduğu gibi kalır.
agent-error-import-hifi-target = Hi-fi içe aktarma paket ya da tema almaz: onları kaldırın ya da retrofit kipinde içe aktarın.
agent-error-import-extension = Yalnızca bir PowerPoint .pptx dosyası içe aktarılabilir.
agent-error-a2ui-unknown = { $name }, render'ın kabul ettiği bir bileşen değil. Bileşenler: { $known }.
agent-error-a2ui-elsewhere = render { $name } öğesini göstermez: { $tool } kullanın.
agent-error-a2ui-host-only = { $name } uygulamaya ayrılmıştır: render onu gösteremez.
agent-error-themes-deck-mode = Tema seçici yalnızca slaytlar düzenlenirken açılır.
agent-error-packs-deck-mode = Sununun paket paneli yalnızca slaytlar düzenlenirken açılır.
agent-error-usage-no-deck = Bu sekmenin kullanımı hiçbir sunuya ait değil: bir sunuya yalnızca o sunudan (kullanıcı ya da ajan tarafından) açılan paket veya tema sekmesi ya da sununun kendi paketi sayılır.
agent-error-rewrite-text = Yeniden yazım metin olmadan döndü.
agent-error-paragraphs = Yeniden yazımın paragrafları geçersiz.
agent-error-draft-too-long = Taslak çok uzun.
agent-error-report-elsewhere = Bu pencere o sekmeyi göstermiyor.
agent-error-ui-too-large = Sayfa bağlamı çok büyük.
agent-error-seq-exhausted = Sayfa raporunun numarası aralığın dışında.
agent-error-draft-origin = Taslak reddedildi: bilinmeyen görünüm.
agent-error-draft-no-agent = Bu sekmenin ajanı henüz başlamadı: taslak reddedildi.
agent-error-too-many-images = Taslakta zaten tüm görseller var.
agent-error-image-not-base64 = Yapıştırılan görsel eksiksiz ulaşmadı.
agent-error-image-gone = Yapıştırılan bir görsel artık kullanılamıyor: yeniden yapıştırın.
agent-error-unknown-agent = Bu ajan artık yapılandırmada değil.
agent-error-agent-locked = Ajan seçimi ortam tarafından belirleniyor.
agent-error-agent-not-saved = Ajan seçimi kaydedilemedi.
agent-error-unknown-model = Bu ajan o modeli sunmuyor.
agent-error-model-not-saved = Model seçimi kaydedilemedi.
voice-error-no-dictation = Dikte burada kullanılamıyor
voice-error-busy = Dikte sürüyor: boşta olduğunda yeniden deneyin
voice-error-unknown-model = Bilinmeyen dikte modeli
voice-error-unavailable = Bu model seçilen dili desteklemiyor
voice-error-installed = Bu model zaten kurulu
voice-error-not-downloadable = Bu model indirilmez
voice-error-nothing-to-cancel = Devam eden bir indirme yok
voice-error-bad-silence = Sessizlik 0,5 ile 10 saniye arasında sürmelidir
voice-error-bad-language = Bilinmeyen dikte dili
voice-error-bad-url = API adresi https:// ile başlamalıdır (http:// yalnızca bu bilgisayar için), kullanıcı adı, parametre veya özel karakter olmadan
voice-error-bad-key-env = Anahtar değişkeni OPENAI_API_KEY gibi bir ad olmalıdır: büyük harfler, rakamlar ve _
voice-error-bad-api-model = API model adı yalnızca harf, rakam ve . _ : / - içerebilir; Azure dağıtım adı yalnızca harf, rakam ve . _ -
voice-error-bad-api-version = Azure API sürümü yalnızca harf, rakam, . ve - içerebilir, en çok 32 karakter, örneğin 2025-03-01-preview
voice-error-bad-api-headers = Başlıklar Ad: değer; Ad: $VARIABLE biçiminde, en çok 8 tane yazılır. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection ve yinelenenler reddedilir
voice-error-api-reserved = API adresini, anahtar değişkenini, Azure sürümünü ve başlıkları yalnızca kullanıcı ayarlar: kutuyu show_dictation ile açın ve ondan isteyin
voice-error-hands-free-reserved = Eller serbest dinlemeyi yalnızca kullanıcı açar: kutuyu show_dictation ile açın ve ondan isteyin
voice-error-changed = Ayarlar sorudan bu yana değişti: yeniden sorun
voice-error-nothing-to-download = Bu ajan için indirilecek bir şey yok
voice-error-bad-rate = Konuşma hızı 0,5 ile 2 arasında olmalıdır
voice-error-unknown-voice = Arayüz dili için bilinmeyen ses
voice-error-speech-unavailable = Sesli okuma bu bilgisayarda kullanılamıyor
agent-tune-prompt = İçe aktarılan sunumu ince ayarla
agent-settings-unknown-selected = agent.selected = { $selected } ayarı [agent.presets] içindeki hiçbir ajanı göstermiyor (bilinenler: { $known })
agent-settings-preset-launch = [agent.presets.{ $id }] ajanı command veya builtin değerlerinden tam olarak birine ihtiyaç duyar

# Ajan: yüzeyler, onaylar, ileti dizisi.

agent-surface-cancel = İptal
agent-surface-confirm = Onayla
agent-surface-confirm-danger = Evet, yap
agent-surface-rewrite = Yeniden ifade etme
agent-surface-pick = { $n } seç
agent-surface-keep = Orijinali koru
agent-surface-edit = Uygulamadan önce düzenle
agent-surface-close = Kapat
agent-surface-themes = Temalar
agent-surface-variants = Varyantlar
agent-surface-permission-always = her zaman için
agent-surface-permission-title = Ajan bir izin istiyor
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

agent-not-found = Ajan bulunamadı: { $command }. Ayar [agent.presets].
agent-spawn-failed = Ajan başlatılamadı: { $command }. Ayar [agent.presets].
agent-no-http = Bu ajan Oku Slides'ın araçlarına bağlanamıyor.
agent-local-no-program = Yerel ajan yalnızca Oku Slides uygulamasının içinde çalışır.
agent-local-needs-download = Yerel ajanın önce modelini ({ $gigabytes } GB) indirmesi gerekiyor.
agent-local-intel-mac = Yerel ajan, Apple silikonlu bir Mac gerektirir.
agent-local-unsupported-os = Yerel ajan bu sistemde henüz kullanılamıyor.
agent-model-download = Modeli indir
agent-model-downloading = Model indiriliyor… %{ $percent }
agent-model-cancel = İptal
agent-model-download-failed = Model indirilemedi.
agent-model-download-integrity = İndirilen model bozuktu ve silindi.
agent-model-download-incomplete = Eksik indirme: { $expected } baytın { $got } baytı alındı.
agent-model-download-cancelled = İndirme iptal edildi.
local-agent-no-route = Yerel ajan ne yapacağını anlamadı. Başka bir şekilde söylemeyi deneyin.
local-agent-tool-limit = Yerel ajan { $n } adımdan sonra bitirmeden durdu.
local-agent-invalid-call = Yerel ajan araçları doğru kullanamadı.
local-agent-engine-failed = Yerel ajanın modeli başarısız oldu.
local-agent-retry = Yeniden deneyin ya da ajan panelinden başka bir ajan seçin.
agent-stopped = Ajan durdu.
agent-failed = Ajan isteği reddetti: { $message }
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
agent-error-tab-no-turn = Paket veya tema sekmesini yalnızca kullanıcıyı yanıtlarken açın veya gösterin.
agent-error-tab-limit = Bu turda zaten bir paket veya tema sekmesi açtınız ya da gösterdiniz; orada bitirin ya da başka birine geçmeden önce kullanıcıya sorun.
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
agent-error-no-file = Sunumun henüz bir dosyası yok: mutlak bir yol verin.
agent-error-path-relative = Yol mutlak olmalıdır.
agent-error-path-parent = Yol “..” içermemelidir.
agent-error-setting-locked = { $key } ayarı aracı tarafından değiştirilemez.
agent-error-setting = { $key } ayarı değiştirilemedi: değerini kontrol edin.
agent-error-history-index = Geçmişte gezinmek için bir sürümün sırası gerekir.
agent-error-no-pack = Bu sunum hiçbir paketi sabitlemiyor: pack verin (bkz. packs).
agent-error-history-not-opened = Sekme geçmişi zamanında açmadı: yeniden isteyin.
agent-error-present = Burada sunulacak bir şey yok: slayt yok ya da sekme paket, geçmiş veya salt okunur modunda.
agent-error-locale = Bilinmeyen dil kodu. Sunulan bir dilin kodunu ya da system verin.
agent-error-empty-interface = Değiştirilecek bir şey yok: scheme, locale ya da guides verin.
agent-error-empty-dictation = Değiştirilecek bir şey yok: dikte ayarlarından en az birini verin.
agent-error-not-shown = Bu sekme penceresinde gösterilen sekme değil: kullanıcı onu gösterdiğinde yeniden isteyin.
agent-error-themes-deck-mode = Tema seçici yalnızca slaytlar düzenlenirken açılır.
agent-error-packs-deck-mode = Sununun paket paneli yalnızca slaytlar düzenlenirken açılır.
agent-error-usage-no-deck = Bu sekmenin kullanımı hiçbir sunuya ait değil: bir sunuya yalnızca o sunudan (kullanıcı ya da ajan tarafından) açılan paket veya tema sekmesi ya da sununun kendi paketi sayılır.
agent-error-rewrite-text = Yeniden yazım metin olmadan döndü.
agent-error-paragraphs = Yeniden yazımın paragrafları geçersiz.
agent-error-draft-too-long = Taslak çok uzun.
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
voice-error-changed = Ayarlar sorudan bu yana değişti: yeniden sorun
voice-error-nothing-to-download = Bu ajan için indirilecek bir şey yok
agent-tune-prompt = İçe aktarılan sunumu ince ayarla
agent-settings-unknown-selected = agent.selected = { $selected } ayarı [agent.presets] içindeki hiçbir ajanı göstermiyor (bilinenler: { $known })
agent-settings-preset-launch = [agent.presets.{ $id }] ajanı command veya builtin değerlerinden tam olarak birine ihtiyaç duyar

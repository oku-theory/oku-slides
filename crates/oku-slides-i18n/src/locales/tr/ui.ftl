# Uygulama arayüzü (React). ui/src/i18n.ts tarafından “messages” isteğiyle yüklenir.
ui-loading = Yükleniyor…

## Belge

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · şablonlar
ui-title-rename = Sunuyu yeniden adlandır
ui-title-field = Sunu başlığı
ui-status-unsaved = kaydedilmemiş değişiklikler
ui-status-saved = kaydedildi
ui-status-never-saved = henüz kaydedilmedi
ui-caption-templates = { $count ->
    [one] { $count } şablon
   *[other] { $count } şablon
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slayt
   *[other] { $count } slayt
} · { $status }
ui-template-default-name = Şablon
ui-template-variant-name = { $name } (varyant)
ui-confirm-quit-templates = Değiştirilen şablonlar kaydedilmeden çıkılsın mı?

## Bildirimler

ui-toast-templates-saved = Şablonlar kaydedildi: { $name }
ui-toast-moved = Sunu taşındı
ui-toast-copied = Kopya kaydedildi
ui-toast-select-zone = Yerleştirmek için bir alan seçin: “{ $text }”
ui-toast-field-gone = Düzenlediğiniz alan artık yok: girdiğiniz metin kayboldu.

## Araç çubuğu

ui-toolbar-undo = Geri al
ui-toolbar-undo-label = Geri al: { $label }
ui-toolbar-redo = Yinele
ui-toolbar-redo-label = Yinele: { $label }
ui-toolbar-grid-hide = Izgarayı gizle
ui-toolbar-grid-show = Izgarayı göster
ui-toolbar-ghosts-hide = Hayaletleri gizle
ui-toolbar-ghosts-show = Hayaletleri göster
ui-divider-pile = Slayt yığınının genişliği
ui-divider-notes = Ajan akışının genişliği
ui-toolbar-theme = Tema
ui-toolbar-theme-default = Varsayılan
ui-toolbar-save-copy = Kopyasını kaydet
ui-toolbar-finish = Bitti
ui-toolbar-templates = Şablonlar
ui-toolbar-open = Aç
ui-merge = PDF ile birleştir…
ui-menu = Menü
ui-menu-move = Taşı…
ui-language = Dil
ui-language-system = Sistem dili
ui-scheme = Görünüm
ui-scheme-system = Sistem
ui-scheme-light = Açık
ui-scheme-dark = Koyu
ui-toolbar-add-label = Slayta ekle
ui-add-kind = { $kind ->
    [card] Kart
    [shape] Şekil
    [chart] Grafik
   *[text] Metin
}
ui-toolbar-remove-template = Şablonu sil
ui-toolbar-remove-slide = Slaytı sil
ui-toolbar-remove-hint = Slayt { $n } · ⌘Z ile geri alın

## Geri alma etiketleri

ui-undo-add = { $kind ->
    [card] Kart ekle
    [shape] Şekil ekle
    [chart] Grafik ekle
   *[text] Metin ekle
}
ui-undo-new-template = Yeni şablon: { $name }
ui-undo-new-slide = Yeni slayt: { $name }
ui-undo-grid = Izgara { $columns } × { $rows }
ui-undo-templates = Şablonlar: { $name }
ui-undo-move-template = Şablonu taşı
ui-undo-move-slide = Slaytı taşı
ui-undo-remove-slide = Slaytı sil
ui-undo-remove = Sil
ui-undo-cut = Kes
ui-undo-paste = Yapıştır
ui-undo-place-on-grid = Izgaraya yerleştir
ui-undo-move-on-grid = Izgarada taşı
ui-undo-resize-on-grid = Izgarada yeniden boyutlandır
ui-undo-change-anchor = Sabitlemeyi değiştir
ui-undo-move = Taşı
ui-undo-resize = Yeniden boyutlandır
ui-undo-edit-slide = Slaytı düzenle
ui-undo-dictation = Dikte
ui-undo-rename-set = Seti yeniden adlandır
ui-undo-rename-deck = Sunuyu yeniden adlandır
ui-undo-theme = Temayı değiştir
ui-undo-rename-template = Şablonu yeniden adlandır
ui-undo-describe-template = Şablonu açıkla
ui-undo-chart-kind = Grafik türünü değiştir
ui-undo-chart-data = Verileri düzenle
ui-undo-chart-color = Seriyi renklendir
ui-undo-chart-stack = { $stacked ->
    [yes] Serileri yığ
   *[no] Serileri ayır
}

## Yeni öğeler

ui-new-text = Yeni metin
ui-new-card-title = Kart başlığı
ui-new-card-detail = Ayrıntı

## Slayt yığını

ui-pile-slides = Slaytlar
ui-pile-templates = Şablonlar
ui-pile-add-slide = + Yeni slayt
ui-pile-add-template = + Yeni şablon
ui-pile-drag-hint = Taşımak için sürükleyin · ⌥↑ ⌥↓
ui-slide-alt = Slayt { $n }

## Sahne

ui-stage-rendering = Oluşturuluyor…
ui-stage-elements = Slayt öğeleri
ui-stage-fields = Slayt alanları
ui-stage-edit-text = Metni düzenle
ui-stage-hint-plain = Sonrakine geçmek için Enter veya Tab · iptal için Esc
ui-stage-hint-rich = Sonrakine geçmek için Tab · onaylamak için ⌘ Enter · madde işareti için “- ” · ⌘B kalın, ⌘I italik, ⌘K bağlantı
ui-field = { $kind ->
    [title] Başlık
    [subtitle] Alt başlık
    [kicker] Üst başlık
    [text] Metin
    [body] Gövde
    [author] Yazar
    [date] Tarih
    [organization] Kuruluş
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Metin
    [card] Kart
    [image] Görsel
    [shape] Şekil
    [group] Grup
   *[other] { $kind }
}
ui-grid-cell = süt. { $cols } · satır { $rows }
ui-ghost-zone = Bölge

## Slayt notları

ui-notes = Slayt araçları
ui-stepper-less = { $label }: bir eksik
ui-stepper-more = { $label }: bir fazla
ui-agent-color-other = Başka renk
ui-agent-thread = Ajan
ui-agent-new-conversation = Yeni konuşma
ui-agent-tools = { $count } araç
ui-draft = Taslak
ui-draft-insert = Ekle
ui-draft-insert-title = Olduğu gibi ekle (⌘⇧↩)
ui-draft-send = Ajana gönder
ui-draft-send-title = Ajana gönder (⌘↩)
ui-draft-discard = Sil
ui-agent-state-idle = Ajan
ui-agent-state-starting = Ajan başlatılıyor…
ui-agent-state-working = Ajan çalışıyor…
ui-agent-stopped = Durduruldu.
ui-agent-ended = Tur bitti ({ $stop }).
ui-agent-stop-max-tokens = Ajan uzunluk sınırına ulaştı.
ui-agent-stop-max-turns = Ajan adım sınırına ulaştı.
ui-agent-stop-refusal = Ajan reddetti.
ui-agent-sheet = Ajanın sorusu
ui-agent-sheet-keys = 1–4 seç · Enter onayla · E düzenle · Esc iptal
ui-agent-variants = Varyantlar
ui-agent-drawer-keys = Tıkla veya 1–4 · Esc kapatır
ui-agent-confirm = Ajan onayı

## Dikte

ui-dictation = Dikte
ui-dictation-start = Dikte et
ui-dictation-start-title = Dikte et (⌘D)
ui-dictation-agent-stop = Ajanı durdur (⌘⇧D veya Esc)
ui-dictation-agent-working = Ajan çalışıyor…
ui-dictation-agent-working-hint = ⌘⇧D veya Esc durdurur
ui-dictation-finish = Dikteyi bitir
ui-dictation-finish-title = Bitir (⌘D)
ui-dictation-cancel = İptal
ui-dictation-preparing = Dikte hazırlanıyor…
ui-dictation-heard-nothing = Hiçbir şey duyulmadı: mikrofona biraz daha yakın konuşun
ui-dictation-to-cursor = imleç konumuna
ui-dictation-to-draft = taslağa
ui-dictation-to-field = { $field } alanını değiştir
ui-dictation-to-block = seçili bloktan sonra
ui-dictation-new-block = yeni metin bloğu
ui-dictation-select-zone = bir alan seçin
ui-dictation-downloading = { $name } modeli indiriliyor
ui-dictation-download-progress = %{ $percent } · { $received } / { $total } · yalnızca bir kez
ui-dictation-listening = Dinleniyor · { $time }
ui-dictation-transcribing = Son cümle yazıya dökülüyor…
ui-dictation-stop-in = { $duration } içinde duruyor
ui-dictation-speak = Konuşun, metin burada görünür…
ui-dictation-auto-stop = { $duration } sessizlikten sonra durur
ui-dictation-to-download = İndirilecek: { $size }
ui-dictation-settings = Dikte ayarları
ui-dictation-model-title = Tanıma modeli
ui-dictation-model-intro = Whisper, bu bilgisayarda (CTranslate2). Her model birden çok dili anlar.
ui-dictation-note-turbo = Önerilen, hızlı ve doğru
ui-dictation-note-large = En doğru, daha yavaş
ui-dictation-note-base = Hafif, küçük makineler için
ui-dictation-installed = yüklü
ui-dictation-not-installed = indirilecek
ui-dictation-language = Konuşulan dil
ui-dictation-language-auto = Otomatik algılama
ui-dictation-note-apple = En hızlısı, sisteme yerleşik
ui-dictation-model-unavailable = Dil desteklenmiyor
ui-dictation-model-intro-apple = Apple, sisteme yerleşik, veya bu bilgisayarda Whisper (CTranslate2). Hepsi birden çok dili anlar.
ui-dictation-language-auto-resolved = Otomatik ({ $language })
ui-dictation-to-download-language = İndirilecek dil
ui-dictation-auto-stop-label = Şu kadar sessizlikten sonra kendiliğinden dur
ui-dictation-silence = Sessizlik süresi
ui-dictation-phrases-note = Yalnızca konuşulan bölümler, siz konuşurken cümle cümle yazıya dökülür.
ui-dictation-download-now = Şimdi indir ({ $size })
ui-dictation-download-language = Dili indir
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } sn
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Şablon seçici

ui-picker-new-slide = Yeni slayt
ui-picker-new-template = Yeni şablon, temel alınan…
ui-picker-set = Şablonlar
ui-picker-set-mine = { $name } · benim
ui-picker-keys = Gezinmek için ← → · eklemek için Enter · kapatmak için Esc
ui-picker-close = Kapat
ui-picker-previous = Önceki şablon
ui-picker-next = Sonraki şablon
ui-picker-list = Slayt şablonları

## Grafik

ui-chart-panel = Grafik
ui-chart-kinds = Grafik türü
ui-chart-kind = { $kind ->
    [line] Çizgi
    [pie] Pasta
    [scatter] Dağılım
    [radar] Radar
    [waterfall] Şelale
   *[bar] Çubuk
}
ui-chart-no-data = Bu grafiğin veri tablosu yok.
ui-chart-options = Grafik seçenekleri
ui-chart-stack = Yığılmış
ui-chart-waterfall-hint = Toplam çubuğu için Toplam'ı işaretleyin.
ui-chart-colors = Seri renkleri
ui-chart-color-of = { $name } rengi
ui-chart-color-auto = Otomatik
ui-chart-color-pick = Renk { $n }
ui-chart-column-name = { $col }. sütunun adı
ui-chart-remove-column = Sütunu sil
ui-chart-cell = Satır { $row }, sütun { $col }
ui-chart-remove-row = Satırı sil
ui-chart-add-row = Satır
ui-chart-add-column = Sütun
ui-chart-series = Seri { $n }

## Dosyalar

ui-file-open-prompt = Açılacak dosyanın yolu (.oku-slides.pdf veya .json)
ui-file-save-prompt = Farklı kaydet (tam yol)

## Sekmeler

ui-tabs = Açık sunumlar
ui-tab-new = Yeni sekme
ui-tab-close = Sekmeyi kapat
ui-tab-unsaved = Kaydedilmemiş değişiklikler
ui-tab-templates = Şablonlar
ui-tab-move-new-window = Yeni pencereye taşı
ui-tab-move-to = { $n }. pencereye taşı
ui-tab-close-others = Diğer sekmeleri kapat
ui-close-cancel = İptal
ui-disk-replaced-question = { $file } diskte başka bir sunumla değiştirildi.
ui-disk-replaced-overwrite = Üzerine yaz
ui-disk-replaced-save-as = Farklı kaydet…
ui-same-deck-question = { $file }, “{ $title }” sekmesinde açık olan sunumun bir sürümü.
ui-same-deck-merge = O sekmede birleştir
ui-same-deck-apart = Ayrı aç
ui-history = Geçmiş
ui-history-version = { $date } sürümü
ui-history-restore = Geri yükle
ui-history-back = Geri dön
ui-history-anonymous = Anonim
ui-history-slider = Sunumun sürümleri
ui-history-removed-since = o zamandan beri silindi
ui-history-changed-since = o zamandan beri değişti
ui-history-added-since = o zamandan beri eklenen slaytlar: { $count }
ui-pack-notice-close = Kapat
ui-widgets-files = Widget dosyaları
ui-widgets-common = Ortak
ui-widgets-new = Yeni widget
ui-widgets-name = Widget adı (küçük harf, rakam ve kısa çizgi)
ui-widgets-name-taken = Bu ad zaten kullanılıyor
ui-widgets-name-letter = Ad bir harfle başlamalıdır
ui-widgets-valid = Geçerli
ui-widgets-errors = Hatalar
ui-widgets-empty = Bu pakette henüz widget yok.
ui-widgets-preview = Widget önizlemesi
ui-widgets-no-preview = Önizleme yok
ui-widgets-unsaved = Kaydedilmedi
ui-widgets-in-error = Hatalı widget'lar
ui-widgets-none-in-error = Hiçbir widget'ta hata yok
ui-widgets-section = Widget'lar
ui-widgets-edit = Widget'ları düzenle
ui-widgets-open = { $name } widget'ını aç
ui-widgets-delete-named = { $name } öğesini sil
ui-widgets-confirm-delete = “{ $name }” widget'ı silinsin mi?
ui-widgets-rename-hint = Yeniden adlandırmak için çift tıklayın veya F2'ye basın
ui-widgets-tabs = Şablon sekmeleri
ui-widgets-templates-tab = Şablonlar
ui-widgets-close-tab = { $name } sekmesini kapat

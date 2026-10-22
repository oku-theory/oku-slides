# Uygulama arayüzü (React). ui/src/i18n.ts tarafından “messages” isteğiyle yüklenir.
ui-loading = Yükleniyor…

## Belge

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · düzen paketi
   *[other] { $name } · widget paketi
}
ui-title-rename = Sunuyu yeniden adlandır
ui-title-field = Sunu başlığı
ui-status-unsaved = kaydedilmemiş değişiklikler
ui-status-saved = kaydedildi
ui-status-never-saved = henüz kaydedilmedi
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } düzen
       *[other] { $count } düzen
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widget
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slayt
   *[other] { $count } slayt
} · { $status }
ui-confirm-quit-pack = Değiştirilen paket kaydedilmeden çıkılsın mı?

## Bildirimler

ui-toast-copied = Kopya kaydedildi
ui-toast-field-gone = Düzenlediğiniz alan artık yok: girdiğiniz metin kayboldu.
ui-toast-edit-replaced = Bu metin siz düzenlerken değişti: sizin sürümünüz onun yerini aldı. Geri al diğerini geri getirir.

## Araç çubuğu

ui-toolbar-undo = Geri al
ui-toolbar-undo-label = Geri al: { $label }
ui-toolbar-redo = Yinele
ui-toolbar-redo-label = Yinele: { $label }
ui-divider-pile = Slayt yığınının genişliği
ui-divider-notes = Ajan akışının genişliği
ui-divider-code = Kod boyutu
ui-divider-band = Ayarların yüksekliği
ui-toolbar-theme = Tema
ui-toolbar-theme-embedded = Dosyanın teması
ui-toolbar-theme-deck-group = Bu sunu
ui-toolbar-finish = Bitti
ui-pack-deck-only = Yalnızca bu sunuya kaydedilir
ui-toolbar-open = Aç
ui-menu = Menü
ui-toolbar-add-label = Slayta ekle
ui-add-kind = { $kind ->
    [card] Kart
    [shape] Şekil
    [chart] Grafik
    [table] Tablo
   *[text] Metin
}
ui-add-kind-description = { $kind ->
    [card] Çerçeve içinde bir başlık ve kısa bir metin
    [shape] Renkli bir şekil
    [chart] Bir veri tablosundan çizilen grafik
    [table] Bir veri tablosu
   *[text] Bir metin paragrafı
}
ui-toolbar-remove-slide = Slaytı sil
ui-toolbar-remove-hint = Slayt { $n } · ⌘Z ile geri alın

## Geri alma etiketleri

ui-undo-add = { $kind ->
    [card] Kart ekle
    [shape] Şekil ekle
    [chart] Grafik ekle
    [table] Tablo ekle
   *[text] Metin ekle
}
ui-undo-add-widget = “{ $name }” ekle
ui-undo-new-slide = Yeni slayt: { $name }
ui-undo-move-slide = Slaytı taşı
ui-undo-remove-slide = Slaytı sil
ui-undo-remove = Sil
ui-undo-paste = Yapıştır
ui-undo-place-on-grid = Izgaraya yerleştir
ui-undo-move-on-grid = Izgarada taşı
ui-undo-resize-on-grid = Izgarada yeniden boyutlandır
ui-undo-change-anchor = Sabitlemeyi değiştir
ui-undo-move = Taşı
ui-undo-resize = Yeniden boyutlandır
ui-undo-edit-slide = Slaytı düzenle
ui-undo-rename-pack = Paketi yeniden adlandır
ui-undo-rename-deck = Sunuyu yeniden adlandır
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
ui-pile-layouts = Düzenler
ui-pile-add-slide = + Yeni slayt
ui-pile-drag-hint = Taşımak için sürükleyin · ⌥↑ ⌥↓
ui-slide-alt = Slayt { $n }

## Sahne

ui-stage-rendering = Oluşturuluyor…
ui-stage-widget-pack = Yığından bir widget açın ya da yeni bir tane oluşturun.
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
    [chart] Grafik
    [table] Tablo
    [stack] Yığın
   *[other] { $kind }
}
ui-grid-cell = süt. { $cols } · satır { $rows }
ui-ghost-zone = Bölge

## Slayt notları

ui-notes = Slayt araçları
ui-stepper-less = { $label }: bir eksik
ui-stepper-more = { $label }: bir fazla
ui-list-item = öğe
ui-list-add = { $item } ekle
ui-list-remove = { $item } kaldır
ui-list-move = { $item } taşı
ui-list-full = Liste dolu ({ $max })
ui-undo-list-move = { $item } taşı
ui-agent-color-other = Başka renk
ui-agent-thread = Ajan
ui-agent-new-conversation = Yeni konuşma
ui-agent-undo-turn = Bu turu geri al
ui-agent-detach = Ajanı ayrı bir pencerede aç
ui-agent-attach = Ajanı ana pencereye geri getir
ui-agent-window-empty = Bu sekme için ajan yok
ui-agent-kept-in-draft = Yazılacak yer yok: metin taslakta tutuldu
ui-agent-busy-elsewhere = Ajan şurada yanıt veriyor: { $place }…
ui-agent-scope-slides = slaytlar
ui-agent-scope-atelier = atölye
ui-draft = Taslak
ui-draft-send = Ajana gönder
ui-draft-send-title = Ajana gönder (⌘↩)
ui-draft-discard = Sil
ui-agent-state-idle = Ajan
ui-agent-state-starting = Ajan başlatılıyor…
ui-agent-state-working = Ajan çalışıyor…
ui-agent-dock = Devam ediyor
ui-agent-dock-settings = Ayarlar
ui-agent-jump = ↓ Geçerli tur
ui-agent-tool-input = Argümanlar
ui-agent-tool-output = Sonuç
ui-agent-sheet = Ajanın sorusu
ui-agent-sheet-keys = 1–4 seç · Enter onayla · E düzenle · Esc iptal
ui-agent-variants = Varyantlar
ui-agent-drawer-keys = Tıkla veya 1–4 · Esc kapatır
ui-agent-confirm = Ajan onayı

## Dikte

ui-dictation = Dikte
ui-dictation-start = Dikte et
ui-dictation-start-title = Dikte et (⌘D)
ui-dictation-type = Ajana yaz
ui-dictation-type-title = Ajana yaz (⌘K)
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
ui-dictation-downloading = { $name } modeli indiriliyor
ui-dictation-download-progress = %{ $percent } · { $received } / { $total } · yalnızca bir kez
ui-dictation-listening = Dinleniyor · { $time }
ui-dictation-transcribing = Son cümle yazıya dökülüyor…
ui-dictation-stop-in = { $duration } içinde duruyor
ui-dictation-speak = Konuşun, metin burada görünür…
ui-dictation-auto-stop = { $duration } sessizlikten sonra durur
ui-dictation-to-download = İndirilecek: { $size }
ui-dictation-settings = Ayarlar
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
ui-picker-set-mine = { $name } · benim
ui-picker-set-received = { $name } · alınan
ui-picker-keys = Gezinmek için ← → · eklemek için Enter · kapatmak için Esc
ui-picker-close = Kapat
ui-picker-list = Düzenler
ui-widget-picker-filter = Ada, etikete veya açıklamaya göre filtrele
ui-widget-picker-empty = “{ $query }” ile eşleşen widget yok.
ui-widget-picker-list = Widget'lar
ui-widget-picker-keys = Gezinmek için oklar · Eklemek için Enter · Kapatmak için Esc

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

## Tablo

ui-table-panel = Tablo
ui-table-options = Tablo seçenekleri
ui-table-header-row = Başlık satırı
ui-table-header-column = İlk sütun başlık olarak
ui-table-striped = Çizgili satırlar
ui-table-alignment = Sütun hizalaması
ui-table-align-of = { $name } hizalaması
ui-table-align = { $align ->
    [start] Baş
    [center] Orta
    [end] Son
   *[auto] Otomatik
}
ui-table-column = Sütun { $n }
ui-table-no-data = Bu tabloda veri yok.
ui-undo-table-option = Tablo seçenekleri
ui-undo-table-align = Sütunu hizala
ui-undo-table-data = Tabloyu düzenle
ui-show-as = Şu olarak göster
ui-show-as-table = Tablo olarak göster
ui-show-as-chart = Grafik olarak göster
ui-undo-show-as-table = Tablo olarak göster
ui-undo-show-as-chart = Grafik olarak göster

## Dosyalar

ui-file-open-prompt = Açılacak dosyanın yolu (.oku-slides.pdf, .json veya .pptx)
ui-file-filter-presentations = Sunumlar
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Farklı kaydet (tam yol)

## Sekmeler

ui-tabs = Açık sunumlar
ui-tab-new = Yeni sekme
ui-tab-close = Sekmeyi kapat
ui-tab-unsaved = Kaydedilmemiş değişiklikler
ui-tab-pack = Paket
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
ui-help = Yardım
ui-help-caption = Slayt { $index } / { $count } · salt okunur
ui-slide-previous = Önceki slayt
ui-slide-next = Sonraki slayt
ui-present = Sunum
ui-present-slide = Slayt { $index } / { $count }
ui-present-screen = Tam ekranda sun
ui-toolbar-params-hide = Ayarları gizle
ui-toolbar-params-show = Ayarları göster
ui-help-close = Yardımı kapat
ui-help-search = Yardımda ara
ui-help-search-placeholder = Ara…
ui-help-search-empty = Sonuç yok
ui-help-search-count = { $count } sonuç
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
ui-widgets-field-bound = Sunuya bağlı: burada salt okunur
ui-widgets-source = Kaynak kod
ui-widgets-unsaved = Kaydedilmedi
ui-widgets-in-error = Hatalı widget'lar
ui-widgets-none-in-error = Hiçbir widget'ta hata yok
ui-widgets-section = Widget'lar
ui-widgets-open = { $name } widget'ını aç
ui-widgets-delete-named = { $name } öğesini sil
ui-widgets-confirm-delete = “{ $name }” widget'ı silinsin mi?
ui-widgets-rename-hint = Yeniden adlandırmak için çift tıklayın veya F2'ye basın
ui-pack-tabs = Paket sekmeleri
ui-pack-layouts-tab = Düzenler
ui-theme-tab = Tema
ui-theme-picker = Tema
ui-theme-name = Tema adı
ui-theme-unsaved = Tema değişiklikleri kaydedilmedi
ui-widgets-close-tab = { $name } sekmesini kapat
ui-widgets-removed-by-agent = Ajan { $name } widget'ını sildi.

## İçe aktar

ui-import-title = PowerPoint sunumunu içe aktar
ui-import-mode = İçe aktarma modu
ui-import-hifi = Aslına uygun kopya
ui-import-hifi-note = Bu sunum için hazırlanmış bir tema ve düzenlerle orijinalin görünümünü korur.
ui-import-retrofit = Düzenlerimi kullan
ui-import-retrofit-note = Slaytları kitaplıktaki düzen paketleri ve bir temayla yeniden kurar.
ui-import-layout-packs = Düzen paketleri
ui-import-theme-default = Varsayılan tema
ui-import-no-packs = Kitaplıkta düzen paketi yok.
ui-import-tune = Ajanla ince ayar yap
ui-import-tune-note = Ajan her slaytı orijinalle karşılaştırır ve farkları düzeltir.
ui-import-tune-note-retrofit = Ajan her slaytı orijinalle karşılaştırır ve düzenlerinizin gösteremediği şekiller için bileşenler tasarlar.
ui-import-tune-unavailable = Kullanılabilir ajan yok: { $reason }
ui-import-submit = İçe aktar
ui-import-busy = İçe aktarılıyor…
ui-update-available = Oku Slides { $version } kullanıma hazır.
ui-update-install = Yükle ve yeniden başlat
ui-update-see-release = Sürümü gör
ui-update-later = Daha sonra
ui-update-installing = Güncelleme indiriliyor… %{ $percent }
ui-update-downloading = Güncelleme indiriliyor…
ui-update-up-to-date = Oku Slides güncel.

ui-shortcut-theme = tema, temalar, theme, themes

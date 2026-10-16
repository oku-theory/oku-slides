cli-reading = { $path } okunuyor
cli-invalid-deck = geçersiz sunu: { $path }
cli-deck-valid = geçerli sunu: { $slides } slayt
cli-deck-issue = sorun: { $issue }
cli-rendered = { $pages } sayfa → { $out } (derleme { $compile }, SVG { $svg })
cli-asset-added = varlık { $path } → { $id }
cli-saved = { $path }: { $pages } sayfa, PDF { $pdf } KB, kaynaklar { $sources } KB
cli-opened = { $path }: { $slides } slayt, { $assets } varlık, { $fonts } yazı tipi
cli-extracted = { $dir } konumuna çıkarıldı
cli-stripped = { $path }: kaynaksız kopya
cli-pdf-modified = PDF, Oku Slides'dan sonra başka bir araçla değiştirilmiş
cli-iterations = --iterations > 0 olmalı
cli-no-block = sunuda düzenlenecek blok yok
cli-blob-missing = kaynak blob'u bulunamadı
cli-roundtrip-mismatch = yeniden okunan sunu kaydedilenden farklı
cli-bench-world = ortam oluşturma (yazı tipleri)   { $time }
cli-bench-first = ilk derleme                      { $time }  ({ $pages } sayfa)
cli-bench-same = değişiklik olmadan yeniden derleme { $time }
cli-bench-edited-text = Düzenlenen metin { $n }
cli-bench-edit = set_text sonrası yeniden derleme { $time }
cli-bench-svg = tüm slaytların SVG'si            { $time }  (slayt başına { $each })
cli-bench-pdf = PDF dışa aktarımı                { $time }  ({ $size } KB)
cli-bench-embed = kaynak blob'unu gömme           { $time }
cli-bench-extract = blob'u geri okuma               { $time }
cli-bench-roundtrip = sunu gidiş-dönüşü              aynı
cli-bench-history = geçmiş adımı                  ortalama { $mean } ms, p50 { $p50 } ms
cli-serving = Oku Slides (dev) adresi: http://{ $host }:{ $port }
cli-serving-token = { $url } adresini açın (belirteç sunucu durana kadar geçerlidir)
cli-models-dir = dikte modelleri: { $dir }
cli-stopped = sunucu durduruldu
cli-not-found = bulunamadı
cli-bad-request = geçersiz istek: { $reason }
cli-sse-full = zaten çok fazla olay akışı açık
cli-forbidden = istek reddedildi: yerel arayüzden gelmiyor
cli-unauthorized = istek reddedildi: erişim belirteci yok ya da yanlış
cli-exposed = { $host } üzerinde dinleniyor: sunucuya ağdan erişilebilir; yalnızca bu başlatmanın belirtecini sunan istemcilere hizmet verilir. Ağa ihtiyacınız yoksa [serve] host değerini 127.0.0.1 olarak bırakın
cli-imported = { $path }: { $slides ->
       [one] { $slides } slayt
      *[other] { $slides } slayt
    }, { $pages ->
       [one] { $pages } sayfa
      *[other] { $pages } sayfa
    }, { $losses ->
       [one] { $losses } kayıp
      *[other] { $losses } kayıp
    }, { $requests ->
       [one] { $requests } widget isteği
      *[other] { $requests } widget isteği
    }
cli-import-label = PowerPoint içe aktarma
cli-import-pack-required = retrofit modunda --pack gerekir: bir şablon paketinin kimliği, klasörü ya da adı
cli-import-unknown-pack = bilinmeyen şablon paketi: { $pack }
cli-import-bad-pack = okunamayan şablon paketi { $pack }: pack.json dosyası bir şablon paketi tanımlamıyor
cli-import-bad-theme = hedef tema okunamıyor: tasarım belirteçleri okunamadı
cli-import-write-failed = içe aktarılan sunu { $path } yazılamadı
cli-import-hifi-no-target = --pack ve --theme yalnızca retrofit modu içindir: yüksek doğruluklu içe aktarma (--mode hifi) kendi paketini ve temasını üretir
cli-import-probe-mismatch = üretilen paket bildirildiği gibi yoklanmadı (widget { $widget }): içe aktarıcıda bir hata
cli-import-probe-limit = oluşturulan paket denetim sınırlarını aşıyor (widget { $widget }): yapılandırmanın [render] bölümünde max_widgets, max_widget_kib veya probe_budget_ms değerini artırın
cli-import-probe-failed = oluşturulan paket işleme süreci tarafından denetlenemedi: { $error }
cli-import-pack-missing = içe aktarma deste için paket üretmedi: bu bir içe aktarma hatasıdır
cli-import-theme-missing = içe aktarma deste için tema üretmedi: bu bir içe aktarma hatasıdır
cli-import-reference-hifi-only = --reference hifi moduna aittir: retrofit içe aktarma bir referans oluşturmayla karşılaştırılmaz
cli-import-bad-threshold = [import] bölümündeki score_threshold sıfır veya daha büyük bir sayı olmalı, { $value } değil
cli-import-scored = { $witness } ile görsel fark: ortalama { $mean }, { $threshold } üzerinde { $above ->
    [0] hiç slayt yok
   *[other] { $above } slayt
}
cli-import-unscored = { $reason }

cli-about = Oku Slides sunularını oluşturur ve saklar
cli-arg-fonts = Ek yazı tipi klasörü (TTF/OTF); varsayılan: ayarlardaki paths.fonts
cli-cmd-check = Bir sunuyu denetle
cli-arg-deck = Sunu (JSON)
cli-cmd-render = Her slaytı öğe kutularıyla birlikte SVG olarak oluştur
cli-arg-input = deck.json veya .oku-slides.pdf dosyası
cli-arg-out = Çıktı klasörü
cli-arg-theme = Sununun temasını değiştir: yerleşik ya da kitaplıktaki bir tema, adıyla
cli-cmd-save = Sunuyu .oku-slides.pdf olarak kaydet (PDF + kaynak blob'u)
cli-arg-output = Yazılacak dosya
cli-arg-assets = Varlık klasörü: her dosya bir sha256:… varlığı olur
cli-arg-embed-fonts = --fonts yazı tiplerini de kaynaklara göm
cli-cmd-open = Bir .oku-slides.pdf dosyasını yeniden aç: sunuyu ve varlıklarını çıkar
cli-arg-extract = deck.json, assets/ ve fonts/ yazılacak klasör
cli-cmd-strip = Bir .oku-slides.pdf dosyasını kaynak blob'u olmadan kopyala
cli-cmd-bench = Derleme ve dışa aktarma sürelerini ölç
cli-arg-iterations = Adım başına ölçüm sayısı
cli-cmd-schema = İşlemlerin JSON şemasını yazdır
cli-cmd-serve = Geliştirme sunucusu: arayüz tarayıcıda, Tauri olmadan
cli-arg-serve-input = Açılacak sunu (.oku-slides.pdf veya .json)
cli-arg-port = Bağlantı noktası; varsayılan: ayarlardaki serve.port
cli-arg-ui = Derlenmiş arayüzün klasörü (ui/dist)
cli-cmd-import = Bir PowerPoint sunusunu (.pptx) .oku-slides.pdf olarak içe aktar
cli-arg-pptx = PowerPoint sunusu (.pptx)
cli-arg-mode = retrofit: içerik mevcut bir şablon paketi ve tema üzerinde; hifi: aslına uygun kopya, onun için üretilen paket ve tema üzerinde
cli-arg-pack = Hedef şablon paketi: kimlik, paket klasörü (pack.json, widgets/) ya da ad
cli-arg-import-theme = Hedef tema: yerleşik ya da kitaplıktaki bir tema, kimliği ya da adıyla; varsayılan: varsayılan tema
cli-arg-report = İçe aktarma raporunu da (JSON) yaz: kayıplar, yedekler, kapsam
cli-arg-no-system-fonts = hifi modunda sununun yazı tiplerini sistem yazı tipleri arasında arama
cli-arg-reference = hifi modunda her slaytı sununun referans oluşturmasıyla karşılaştır: auto (varsayılan, office2pdf), office2pdf, soffice (LibreOffice) ya da karşılaştırma (none)

pdf-unreadable = PDF okunamıyor
pdf-unsupported-version = Bu PDF, Oku Slides'ın daha yeni bir sürümüyle yazılmış
pdf-corrupted = Bu PDF'teki kaynaklar bozuk
pdf-write = PDF yazılamadı

render-compile = Sunu oluşturulamadı
render-pdf = PDF dışa aktarımı başarısız oldu
render-no-page = Bu sayfa mevcut değil
render-image = Slayt görüntüsü oluşturulamadı

render-setup = Oluşturucu başlatılamadı
render-missing-image = eksik görsel
diagnostic-error = hata
diagnostic-warning = uyarı
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = ipucu: { $hint }
render-chart-failed = grafik çizilemedi

registry-bad-pack-id = Geçersiz şablon paketi tanımlayıcısı: { $id }
registry-compile = Şablon paketinin widget'ları derlenemedi
registry-missing = Şablon paketi widget'larını bildirmiyor
render-timeout = Oluşturma { $seconds } sn'den uzun sürdü ve durduruldu
render-out-of-memory = Oluşturma { $mib } MiB'den fazla bellek gerektirdi ve durduruldu
render-worker-lost = Oluşturma işlemi durdu
render-frame = Oluşturma işleminden okunamayan ileti geldi
render-page-too-large = Bu slayt önizleme için çok büyük ({ $mib } MiB'den fazla, çoğu zaman çok büyük bir görsel yüzünden). Yine de PDF veya PNG olarak dışa aktarılabilir.
render-stale = Oluşturma değiştirildi; yeniden deneyin
render-pack-disabled = “{ $pack }” paketinin widget'ları bu oturum için devre dışı
render-protocol = Oluşturma işlemi Oku Slides'ın başka bir sürümüne ait; uygulamayı yeniden başlatın
render-remote = Oluşturma işlemi isteği tamamlayamadı
registry-schema-remote = Geçersiz widget kaydı: { $message }
probe-not-probed = Henüz denetlenmedi: paketin widget'larına ayrılan süre doldu. Bir sonraki işlemede denetlenecek.
probe-too-many-files = Pakette { $max } widget dosyasından fazlası var: bu dosya yüklenmedi.
probe-too-large = { $max } KiB'tan büyük: yüklenmedi.

import-unreadable = Bu sunu okunamıyor
import-not-a-presentation = Bu dosya bir PowerPoint sunusu değil
import-protected = Bu sunu şifreli veya eski .ppt biçiminde: önce .pptx olarak kaydedin
import-too-large = Bu sunu içe aktarılamayacak kadar büyük
import-output-too-large = Ortaya çıkacak sunu, içe aktarma için izin verilen boyutu aşar
import-loss-list-levels = Liste düzeyleri tek düzeye indirildi
import-loss-merged-cells = Birleştirilmiş hücreler ayrıldı
import-loss-percent-stacked = %100 yığılmış grafik yığılmış olarak gösterildi
import-loss-chart-format = Grafiğin { $format } sayı biçimi korunmadı
import-loss-run-style = { $effect ->
    [underline] Altı çizili biçim kayboldu
    [strike] Üstü çizili biçim kayboldu
    [baseline] Üst simge veya alt simge kayboldu
    [caps] Büyük harfler kayboldu
    [font] Bir bölümün yazı tipi kutununkiyle değiştirildi
    [tracking] Bir bölümün harf aralığı kutununkiyle değiştirildi
    [weight] Normal bölüm kutu gibi kalın gösterildi
    [slant] Düz bölüm kutu gibi italik gösterildi
   *[highlight] Vurgulama kayboldu
}
import-loss-transform = Döndürme, çevirme veya kırpma kayboldu
import-loss-color-dropped = { $color } renginin temada yakın bir rengi yok: tema rengi kullanıldı
import-loss-text-truncated = Metin sınırlara sığması için kısaltıldı
import-loss-table-truncated = Tablo sınırlara sığması için kısaltıldı
import-loss-opaque = { $reason ->
    [media] Video veya ses içe aktarılmadı (yedek görüntü yok)
    [ole] Gömülü nesne içe aktarılmadı (yedek görüntü yok)
    [math] Denklem içe aktarılmadı (yedek görüntü yok)
    [smartart] SmartArt içe aktarılmadı (yedek görüntü yok)
   *[unknown] Desteklenmeyen nesne içe aktarılmadı (yedek görüntü yok)
}
import-loss-unsupported-image = { $format } biçimindeki görüntü içe aktarılmadı
import-loss-hidden-slide = Gizli slayt görünür olarak içe aktarıldı
import-loss-effects = Gölgeler, yansımalar ve 3B efektler kayboldu
import-loss-series-dropped = Pasta grafik tek bir seri gösterir: { $kept } korundu, { $dropped } çıkarıldı
import-loss-repeated-decor = { $slides } slaytta tekrarlanan “{ $text }” üst veya alt bilgisi atlandı: tema kendisininkini çizer
import-loss-no-slot = İçerik atlandı: seçilen şablonda ona yer yok
import-loss-page-fitted = Kaynak sayfa ({ $width } × { $height } pt) 16:9 değil: içerik ölçeklendi ve ortalandı
import-loss-paragraph-style = Bir paragrafın hizalaması, satır aralığı veya paragraf aralığı kutununkine uyarlandı
import-loss-shape-style = { $effect ->
    [gradient] Degrade düz renk olarak gösterildi
    [picture] Dolgu resmi şeklin çerçevesine uzatıldı
    [dash] Kesikli çizgi düz gösterildi
    [arrow] Çizginin ok uçları çizilmedi
   *[geometry] Bilinmeyen şekil dikdörtgen olarak gösterildi
}
import-loss-layout-dropped = Bir paket için çok fazla düzen: “{ $layout }” için şablon yok; slaytları boş widget'ı kullanır ve süslemesini öğe olarak alır
import-loss-decor-truncated = Süsleme bir paket dosyasının boyutuna sığması için kısaltıldı: { $omitted } öğe atlandı
import-loss-table-style = Tablo stili yaklaşık: kendi hücre arka planları, kenarlıkları, sütun genişlikleri veya metin boyutları kayboldu
import-loss-overflow = “{ $element }” öğesi slaytın kenarını aşıyor
import-reference-unavailable = Görsel puan yok: { $cause ->
    [timeout] referans oluşturma çok uzun sürdü
    [memory] referans oluşturma çok fazla bellek kullandı
    [pages] referans oluşturmada sunudaki { $deck } sayfaya karşılık { $reference } sayfa var
   *[failed] referans oluşturucu sunuyu okuyamadı
}
import-font-substituted = { $family } yerine uyumlu ölçülere sahip { $by } kullanıldı
import-font-missing = { $family } bulunamadı: metin { $by } yazı tipini kullanıyor
import-font-restricted = { $family }: lisansı bazı stillerin sunuya gömülmesini yasaklıyor
import-font-restricted-substituted = { $family } sunuya gömülemez (lisans): yerine uyumlu ölçülere sahip { $by } kullanıldı
import-font-restricted-missing = { $family } sunuya gömülemez (lisans): metin { $by } yazı tipini kullanıyor
import-font-too-large = { $family }: dosyalar desteye gömülemeyecek kadar büyük; metin { $by } yazı tipini kullanıyor
import-font-missing-no-fallback = { $family } ve temanın gövde yazı tipi bulunamadı: metin motorun varsayılan yazı tipiyle gösterilir
import-warning-effect-ignored = Efekt içe aktarılmadı: { $detail }
import-warning-approximated = Yaklaşık olarak gösterildi: { $detail }
import-warning-unreadable-item = Okunamayan öğe atlandı: { $detail }
import-warning-unreadable-part = Okunamayan bölüm: { $detail }
import-warning-missing-part = Eksik bölüm: { $detail }
import-warning-unsupported-chart = Desteklenmeyen grafik: { $detail }
import-warning-unsupported-image = Desteklenmeyen görüntü biçimi: { $detail }
import-warning-oversized-image = Görüntü içe aktarılamayacak kadar büyük (piksel sayısı): { $detail }
import-warning-external-image = Dosya dışına bağlı görüntü içe aktarılmadı: { $detail }
import-warning-unsafe-svg = SVG görüntüsü güvenli olmayan içerikten arındırıldı veya atıldı: { $detail }
import-warning-unknown-table-style = Bilinmeyen tablo stili: { $detail }
import-warning-unsafe-link = Bağlantı reddedildi: { $detail }
import-warning-nesting-too-deep = İç içe yerleştirme çok derin, kesildi: { $detail }
import-warning-font-embedded-unreadable = Gömülü yazı tipi okunamadı: { $detail }
import-warning-truncated = Çok fazla uyarı: sonrakiler atlandı

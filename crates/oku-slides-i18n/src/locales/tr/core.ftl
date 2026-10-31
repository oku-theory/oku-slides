op-slide-not-found = Slayt bulunamadı: { $subject }
op-element-not-found = Öğe bulunamadı: { $subject }
op-duplicate-id = Tanımlayıcı zaten kullanılıyor: { $subject }
op-index-out-of-range = Konum aralık dışında
op-invalid-value = { $subject } için geçersiz değer
op-invalid-value-because = { $subject } için geçersiz değer: { $reason }
op-value-no-value = parametrenin değeri yok
op-value-no-dataset = grafiğin veri kümesi yok
op-value-grid-tracks = bir ızgarada 1 ile 24 arasında sütun ve satır olur
op-value-bad-item-id = madde kimliği, “/” içermeyen boş olmayan bir metindir
op-value-missing-item-id = her maddenin bir kimliği olmalı
op-value-too-deep = öğeler çok derin iç içe geçmiş
op-value-empty-path = yol boş
op-value-not-text = bir metin veya null bekleniyor
op-value-empty-frame = genişlik ve yükseklik 0'dan büyük olmalı
op-value-blocks-off-grid = akış yuvalarını slayt yerleştirir ve ızgara onları gizler; önce her öğeyi boş bir yuvaya taşıyın
op-value-outside-grid = hücre ızgaranın dışında
op-value-not-tokens = bir nesne veya null bekleniyor
op-value-not-a-slot = bu parametre bir yuva değil
op-value-out-of-bounds = sayı izin verilen aralığın dışında
op-value-unreadable = değer beklenen biçimde değil
op-value-valign = top, middle veya bottom bekleniyor
op-value-path = yol, sayılarıyla birlikte M, L, C, Q ve Z komutlarından oluşur ve M ile başlar
op-unknown-widget = Bilinmeyen bileşen: { $subject }
op-unknown-namespace = Bu sunuda { $pack } paketi yok.
op-pack-required = { $pack } paketi { $by } için gerekli; önce onu kaldırın.
op-pack-in-use = { $pack } paketi hâlâ { $count } öğe tarafından kullanılıyor.
op-pack-requirement-missing = { $pack } paketi { $required } paketine ihtiyaç duyuyor, ancak bu sunuda yok.
op-base-required = Her sunu temel bileşen paketini korur.
op-pack-kind-conflict = Bu sunuda diğer türde (bileşen veya düzen) bir { $pack } paketi zaten var.
op-theme-cycle = Temalar birbirini döngü içinde genişletiyor: { $chain }.
op-theme-closure = Bu sunuda sabitlenen temalar tam olarak sununun teması ve onun üst temaları değil ({ $namespace }).
op-unknown-param = Bilinmeyen parametre: { $subject }
op-param-type-because = { $subject } için geçersiz değer: { $reason }
op-param-shape-text = { $nullable ->
    [yes] bir metin veya null bekleniyor
   *[no] bir metin bekleniyor
}
op-param-shape-choice = { $nullable ->
    [yes] değerlerinden biri veya null bekleniyor
   *[no] değerlerinden biri bekleniyor
}
op-param-shape-paragraphs = { $nullable ->
    [yes] bir paragraf listesi veya null bekleniyor
   *[no] bir paragraf listesi bekleniyor
}
op-param-shape-elements = { $nullable ->
    [yes] bir öğe listesi veya null bekleniyor
   *[no] bir öğe listesi bekleniyor
}
op-param-shape-flag = { $nullable ->
    [yes] bir mantıksal değer veya null bekleniyor
   *[no] bir mantıksal değer bekleniyor
}
op-param-shape-number = { $nullable ->
    [yes] bir sayı veya null bekleniyor
   *[no] bir sayı bekleniyor
}
op-param-shape-bounded-number = { $nullable ->
    [yes] sınırları içinde bir sayı veya null bekleniyor
   *[no] sınırları içinde bir sayı bekleniyor
}
op-param-shape-grid = { $nullable ->
    [yes] bir ızgara veya null bekleniyor
   *[no] bir ızgara bekleniyor
}
op-param-shape-chart-option = { $nullable ->
    [yes] bir grafik seçeneği veya null bekleniyor
   *[no] bir grafik seçeneği bekleniyor
}
op-param-shape-table-option = { $nullable ->
    [yes] bir tablo seçeneği veya null bekleniyor
   *[no] bir tablo seçeneği bekleniyor
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] bir grafik veya tablo seçeneği veya null bekleniyor
   *[no] bir grafik veya tablo seçeneği bekleniyor
}
op-param-shape-items = { $nullable ->
    [yes] bir madde listesi veya null bekleniyor
   *[no] bir madde listesi bekleniyor
}
op-param-shape-list = { $nullable ->
    [yes] bir liste veya null bekleniyor
   *[no] bir liste bekleniyor
}
op-param-shape-add-element-only = { $nullable ->
    [yes] öğeler add_element ile eklenir, veya null
   *[no] öğeler add_element ile eklenir
}
op-param-shape-set-param-only = { $nullable ->
    [yes] değer set_param ile ayarlanır, veya null
   *[no] değer set_param ile ayarlanır
}
op-param-shape-path-option = { $nullable ->
    [yes] bir şekil yolu (d anahtarlı nesne) veya null bekleniyor
   *[no] bir şekil yolu (d anahtarlı nesne) bekleniyor
}
op-item-not-found = { $subject } adlı madde yok
op-list-full = { $subject } listesi dolu (en fazla { $max } madde)
op-list-minimum = { $subject } listesinde en az { $min } madde olmalı
op-list-cell-too-long = { $subject } listesinin bir hücresi çok uzun (sütun { $column })
op-list-id-too-long = { $subject } listesindeki bir maddenin kimliği çok uzun
op-list-too-many-items = { $subject } listesinde çok fazla madde var (en fazla { $max })
op-slot-rejects = { $subject } burada kabul edilmiyor
op-wrong-category = { $subject } burada kullanılamaz
op-choice-out-of-range = { $subject } için izin verilmeyen değer
transaction-refused = { $index }. işlem reddedildi: { $reason }

sources-unreadable = Kaynaklar okunamıyor
sources-not-sources = Bu bir Oku Slides kaynak arşivi değil
sources-unsupported-version = Bu kaynaklar Oku Slides'ın daha yeni bir sürümüyle yazılmış
sources-bad-name = Kaynaklarda reddedilen dosya adı
sources-missing-deck = Kaynaklarda sunu eksik
sources-too-large = Bu dosyadaki kaynaklar açılamayacak kadar büyük
sources-too-deep = Bu sunu öğeleri çok derin iç içe yerleştiriyor

issue-empty-slide-id = boş slayt tanımlayıcısı
issue-duplicate-slide-id = yinelenen slayt tanımlayıcısı
issue-duplicate-element-id = yinelenen öğe tanımlayıcısı
issue-empty-element-id = boş tanımlayıcı
issue-cell-outside-grid = hücre ızgaranın dışında
issue-empty-frame = sıfır boyutlu çerçeve
issue-free-without-frame = çerçevesiz serbest öğe
issue-unknown-widget = bilinmeyen bileşen: { $kind }
issue-wrong-category = { $kind } burada kullanılamaz
issue-unknown-param = bilinmeyen parametre: { $param }
issue-param-type = { $param } için geçersiz değer
issue-choice-out-of-range = { $param } için izin verilmeyen değer
issue-slot-rejects = { $kind } burada kabul edilmez
issue-placed-in-flow = düzenlenmiş bir listede çerçeveyle yerleştirilmiş öğe
issue-unknown-bind = { $param } bilinmeyen bir sunu ayarına bağlı
issue-slot-too-deep = öğeler çok derin iç içe
issue-invalid-style = geçersiz stil değeri: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = eksik varlık: { $asset }
issue-pack-version-mismatch = değiştirilmiş paket sürümü atıldı: { $version }
issue-pack-version-missing = paket sürümü kaynaklarda yok: { $version }
issue-pack-pin-mismatch = Gömülü paket { $pack }, sunumun beklediğiyle eşleşmiyor.
issue-pack-tokens-invalid = Gömülü paket { $pack } geçersiz bir bileşen belirteci içeriyor: { $reason }
issue-pack-kind-mismatch = Gömülü { $pack } paketi, sununun beklediği türde (bileşen veya düzen) değil.
issue-pack-kind-conflict = { $pack } ad alanı hem bileşen paketi hem de düzen paketi olarak sabitlenmiş: yalnızca biri kullanılabilir.
issue-pack-requirement-missing = { $pack } paketi { $required } paketine ihtiyaç duyuyor, ancak bu sunuda yok.
issue-theme-version-mismatch = değiştirilmiş tema sürümü atıldı: { $version }
issue-theme-version-missing = tema sürümü kaynaklarda yok: { $version }
issue-theme-pin-mismatch = Gömülü tema { $namespace }, sunumun beklediğiyle eşleşmiyor.
issue-theme-closure = Bu sunuda sabitlenen temalar tam olarak sununun teması ve onun üst temaları değil ({ $namespace }).
issue-usage-line-invalid = kullanım günlüğünün { $line }. satırı okunamadı, atıldı
issue-usage-line-forged = { $turn } kullanım turu farklı içerikle iki kez kaydedildi, biri atıldı
issue-usage-too-large = kullanım günlüğü çok büyük, atıldı
issue-usage-unreadable = kullanım günlüğü okunamadı, atıldı
issue-usage-lines-dropped = { $count ->
    [one] kullanım günlüğünün { $count } satırı daha atıldı
   *[other] kullanım günlüğünün { $count } satırı daha atıldı
}
usage-range-bad-date = “{ $date }” bir tarih değil: YYYY-AA-GG biçimini kullanın.
usage-range-reversed = Başlangıç tarihi { $since }, bitiş tarihi { $until } tarihinden sonra.
pack-token-entry-invalid = Paket bildiriminde geçersiz bileşen belirteci “{ $path }”.
pack-manifest-json = Paket bildirimi okunamıyor: { $reason }
issue-unsupported-link = desteklenmeyen bağlantı: { $link }
issue-highlight-zero = vurgu renkleri 1'den başlayarak numaralanır
issue-unreadable-element = okunamayan öğe, bir kenara ayrıldı
issue-slot-repaired = bilinen bir yuvası olmayan öğe, { $slot } içine yerleştirildi
issue-unreadable-item = okunamayan liste maddesi, bir kenara ayrıldı
issue-unreadable-row = okunamayan tablo satırı, bir kenara ayrıldı
issue-data-truncated = veri çok büyük, okunurken kırpıldı
issue-unreadable-slide = okunamayan slayt, bir kenara ayrıldı
issue-unreadable-field = okunamayan sunum alanı { $field }, sıfırlandı
op-invalid-chart = { $subject } grafiği: { $reason }
op-invalid-table = { $subject } tablosu: { $reason }

replica-corrupted = Bu dosyadaki sunum geçmişi bozuk
replica-too-large = Bu dosyadaki sunum geçmişi açılamayacak kadar büyük
replica-write = Sunum geçmişi güncellenemedi
replica-unrelated = Bu iki sunumun ortak bir kökeni yok
replica-unsupported-version = Bu sunum, Oku Slides'ın bu sürümünün okuyamadığı bir biçim kullanıyor
replica-too-deep = Bu sunum kaydedilemiyor: silinen öğeler geçmişinde çok derin iç içe geçmiş. Son taşımalarınızı geri alın, ardından yeniden kaydedin.

widget-unreadable = Widget bildirimleri okunamadı
widget-fault-empty-name = Widget { $widget }: bir widget veya parametrenin adı yok
widget-fault-duplicate-widget = { $widget } widget'ı iki kez bildirildi
widget-fault-duplicate-param = { $widget } widget'ı aynı parametreyi iki kez bildiriyor
widget-fault-empty-choice = Widget { $widget }: bir seçimin değeri yok
widget-fault-default-not-in-choice = Widget { $widget }: bir seçimin varsayılan değeri kendi değerleri arasında değil
widget-fault-default-wrong-type = Widget { $widget }: bir varsayılan değer, parametresinin türüyle eşleşmiyor
widget-fault-bind-not-plain = Widget { $widget }: yalnızca düz metin parametreleri sunuma bağlanabilir
widget-fault-unknown-slot-kind = Widget { $widget }: bir slot bilinmeyen bir öğe widget'ını kabul ediyor
widget-fault-empty-list = Widget { $widget }: bir liste en az bir sütun gerektirir
widget-fault-column-type = Widget { $widget }: bir liste sütunu plain, rich, number, flag, choice veya asset (veya iç içe bir p.list, tek düzey) olabilir; madde başına serbest içerik için alt widget ile p.slot kullanın
widget-fault-reserved-name = Widget { $widget }: ayrılmış ad: id, kind veya / içeren bir ad
widget-fault-list-bounds = Widget { $widget }: bir listenin maksimumu minimumunun altında
widget-fault-list-default-too-large = Widget { $widget }: bir listenin varsayılan değeri liste boyut sınırlarını aşıyor
widget-fault-list-depth = Widget { $widget }: iç içe bir liste, liste sütunu içeremez; listeler yalnızca tek düzey iç içe olabilir
widget-fault-list-default-below-min = Widget { $widget }: iç içe bir listenin varsayılan değeri, alt sınırından daha az öğe içeriyor
widget-fault-slide-sizing = Widget { $widget } bir slayt: grow veya typography yalnızca öğe widget'ları tarafından bildirilir
widget-fault-pack-category = Widget { $widget } paketine uymuyor: düzen paketleri slayt widget'larını, widget paketleri öğe widget'larını içerir
widget-fault-unqualified = Widget { $widget } bir paket ad alanına sahip değil
issue-empty-item-id = { $param } listesinde tanımlayıcısı olmayan bir madde var
issue-duplicate-item-id = { $param } listesinde yinelenen bir madde tanımlayıcısı var
issue-invalid-item-id = { $param } listesinde eğik çizgi içeren bir öğe kimliği var
issue-list-bounds = { $param } listesinde çok az ya da çok fazla madde var

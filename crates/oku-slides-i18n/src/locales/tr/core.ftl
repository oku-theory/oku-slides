op-slide-not-found = Slayt bulunamadı: { $subject }
op-element-not-found = Öğe bulunamadı: { $subject }
op-duplicate-id = Tanımlayıcı zaten kullanılıyor: { $subject }
op-index-out-of-range = Konum aralık dışında
op-invalid-value = { $subject } için geçersiz değer
op-invalid-value-because = { $subject } için geçersiz değer: { $reason }
op-value-no-value = parametrenin değeri yok
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
knowledge-path-empty = yol boş
knowledge-path-not-absolute = yol / ile başlamalı
knowledge-path-bad-segment = bir segment boş, nokta ile bitiyor ya da a-z, 0-9, nokta, alt çizgi ve tire dışında bir karakter kullanıyor
knowledge-path-dot-segment = bir segment nokta ile başlamamalı
knowledge-path-reserved = bir segment ayrılmış bir sistem adı
knowledge-path-too-deep = yolda çok fazla segment var
knowledge-path-too-long = yol çok uzun
knowledge-path-not-markdown = dosya adı .md ile bitmeli
knowledge-frontmatter-missing = dosya --- ile sınırlanmış bir başlık bloğuyla başlamalıdır
knowledge-frontmatter-unterminated = başlık bloğunun kapanış --- satırı yok
knowledge-frontmatter-too-large = başlık bloğu çok büyük
knowledge-frontmatter-not-yaml = başlık geçerli bir YAML değil
knowledge-frontmatter-not-mapping = başlık, metin anahtarlı bir eşleme olmalıdır
knowledge-frontmatter-duplicate-key = "{ $key }" anahtarı başlıkta iki kez geçiyor
knowledge-frontmatter-alias = başlıkta YAML çapaları veya takma adları kullanılamaz
knowledge-frontmatter-tag = başlıkta YAML etiketleri kullanılamaz
knowledge-frontmatter-too-deep = başlık çok derin iç içe
knowledge-frontmatter-after-document-end = başlıkta YAML belge sonu ... sonrasında metin var
knowledge-frontmatter-flow-mapping = başlık tek bir blok eşlemesi olmalı, satır başına bir anahtar, `...` sonu olmadan
knowledge-frontmatter-type-missing = başlıkta boş olmayan bir tür gerekir
knowledge-frontmatter-bad-timestamp = başlıktaki "{ $key }" değeri saat dilimi farkı olan bir RFC 3339 tarih-saat olmalıdır
knowledge-frontmatter-not-allowed = "{ $key }" anahtarına bu dosyanın başlığında izin verilmez
knowledge-reason-not-found = bu yolda dosya yok
knowledge-reason-exists = bu yolda zaten bir dosya var
knowledge-reason-id-missing = yeni bir dosyanın kimliği olmalı
knowledge-reason-id-invalid = dosya kimliği 32 küçük harfli onaltılık basamaktan oluşur
knowledge-reason-id-taken = bu kimlik başka bir dosyada zaten var
knowledge-reason-id-mismatch = bu yoldaki dosyanın kimliği farklı
knowledge-reason-edit-empty-old = değiştirilecek metin boş olamaz
knowledge-reason-edit-no-match = değiştirilecek metin dosyada yok
knowledge-reason-edit-ambiguous = değiştirilecek metin dosyada birden çok kez geçiyor
knowledge-reason-too-many-files = pakette çok fazla dosya var
knowledge-reason-file-too-large = dosya çok büyük
knowledge-reason-total-too-large = paket çok büyük
knowledge-reason-over-limit = bilgi tabanı sınırlarını aştı: yalnızca dosya silmek onu yeniden sınırların altına indirir
knowledge-no-text-layer = bu PDF'nin metin katmanı yok (bir tarama): metinle dışa aktarın ya da metnini yapıştırın
knowledge-legacy-format = bu eski Office biçimi okunamıyor: .pptx, .docx veya .xlsx olarak kaydedin
knowledge-document-encrypted = bu belge parolayla korunuyor: korumasız bir kopyasını kaydedin
knowledge-unsupported-document = bu tür bir belge içe aktarılamaz: PDF, DOCX, PPTX, XLSX, ODS, Markdown veya metin
knowledge-document-damaged = bu belge hasarlı ve okunamıyor
knowledge-document-type-mismatch = bu dosyanın içeriği adıyla uyuşmuyor
knowledge-document-too-large = bu belge içe aktarmak için çok büyük
op-knowledge = Bilgi dosyası { $path }: { $reason }
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
deck-inline-data = { $element } öğesi verilerini seçeneklerinde saklıyor: bu sunum dosyası güncel biçimde değil
sources-not-sources = Bu bir Oku Slides kaynak arşivi değil
sources-unsupported-version = Bu kaynaklar Oku Slides'ın daha yeni bir sürümüyle yazılmış
sources-bad-name = Kaynaklarda reddedilen dosya adı
sources-missing-deck = Kaynaklarda sunu eksik
sources-too-large = Bu dosyadaki kaynaklar açılamayacak kadar büyük
sources-too-deep = Bu sunu öğeleri çok derin iç içe yerleştiriyor
sources-base-refused = Bu sunumun bir veri dosyası bozuk veya kaydedilemedi

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
pack-manifest-order = Paketin öğe sırası “{ $name }” noktasında geçersiz.
pack-manifest-data-count = Paket { $count } veri içeriyor, en fazla { $max }.
pack-manifest-data-base = Paketin “{ $data }” verisi harici bir dosyaya başvuruyor; bir paketin verileri kendi hücrelerini içerir.
pack-manifest-data-cells = Paketin verileri { $count } hücre içeriyor, en fazla { $max }.
issue-unsupported-link = desteklenmeyen bağlantı: { $link }
issue-highlight-zero = vurgu renkleri 1'den başlayarak numaralanır
issue-unreadable-element = okunamayan öğe, bir kenara ayrıldı
issue-slot-repaired = bilinen bir yuvası olmayan öğe, { $slot } içine yerleştirildi
issue-unreadable-item = okunamayan liste maddesi, bir kenara ayrıldı
issue-unreadable-row = okunamayan tablo satırı, bir kenara ayrıldı
issue-data-truncated = “{ $data }” verisi çok büyük, okunurken kırpıldı ve salt okunur tutuldu
issue-data-name-taken = “{ $data }” veri adı zaten kullanılıyor
issue-data-cell-type = “{ $data }” verisi: “{ $column }” sütunu { $type } türünde olmayan değerler içeriyor
issue-data-too-many = sunuda çok fazla veri var ({ $count }); okunmayanlar: { $dropped }
issue-data-base-missing = “{ $data }” verisi: temel dosyası eksik, salt okunur tutuldu
issue-data-edits-lost = “{ $data }” verisi: değiştirilen bir tabloya yapılan düzenlemeler birleştirmede kayboldu
issue-data-inline-dropped = { $element } öğesi: eski bir sürüm grafiğinin veya tablosunun verilerini seçeneklerinde saklamış; bunlar kaldırıldı
issue-data-kind-unknown = “{ $data }” verisi: bilinmeyen tür “{ $kind }”, belki daha yeni bir sürümden; okunmadı
issue-data-query-cycle = “{ $data }” verisi: sorgusu kendini doğrudan ya da diğer sorgular üzerinden yeniden okuyor; hesaplanmadı
issue-data-query-missing-input = “{ $data }” verisi: sorgusu destede olmayan “{ $input }” adını okuyor; hesaplanmadı
issue-data-query-invalid = “{ $data }” verisi: sorgusu geçersiz ({ $reason }); hesaplanmadı
issue-data-query-failed = “{ $data }” verisi: sorgusu başarısız oldu ({ $reason }); sonuç yok
issue-more = gösterilmeyen diğer sorunlar: { $count }
op-data-deck-full = Sununun verileri sınırını aşacaktı: en fazla { $max } { $what ->
    [row-ids] satır kimliği
    [overlay] düzenlenmiş hücre
    [text] bayt metin
   *[data] veri
}
op-data-missing = “{ $data }” verisi bulunamadı
op-data-name-taken = “{ $name }” veri adı zaten kullanılıyor
op-data-column-in-use = “{ $column }” sütununu { $views } gösteriyor; önce onları başka sütunlara bağlayın (columns ile bind_data)
op-data-in-use = Bu veriyi { $views }{ $more ->
    [0] {""}
   *[other] {" "}ve { $more } görünüm daha
}{" "}gösteriyor; önce bunları kaldırın veya başka bir veriye bağlayın
op-data-column-missing = “{ $data }” verisinde “{ $column }” sütunu yok
op-data-not-view = “{ $param }” parametresi veri gösteremez: bunu yalnızca bir grafik veya tablo seçeneği yapabilir
op-data-truncated = “{ $data }” verisi okunurken kırpıldı ve salt okunur; düzenlemek için tablosunun tamamını değiştirin
op-data-rebind = { $subject } öğesi zaten başka verileri gösteriyor; değiştirmek için bind_data kullanın
op-data-internal = Bir toplu işlem bir tablonun tabanını veya katmanını ayarlayamaz
op-data-batch-too-large = Toplu işlem çok büyük: hücre sayısı { $count }, en fazla { $max }
op-data-inline = { $subject } öğesi verilerini seçeneklerinde taşıyor; bunları add_data ile sunum verisi olarak ekleyin ve bind_data ile bağlayın
op-data-derived = “{ $data }” verisi bir sorgudan türetilir: satırlarını değil, sorgusunu düzenleyin
op-data-source = Bir veri ya tablo ya da sorgu (sql) alır, tam olarak birini
op-data-cycle = “{ $data }” verisi sorguları üzerinden kendini yeniden okurdu; döngüyü kaldırın
op-data-unknown-input = Sorgu, destenin verisi olmayan “{ $name }” adını okuyor
op-data-not-query = “{ $data }” verisi bir sorgu değil; tablosunu edit_data ile düzenleyin
op-data-read-by-queries = Bu veriyi { $queries } sorguları okuyor{ $shown ->
    [0] {""}
   *[other] {" "}ve { $views } gösteriyor
}{ $more ->
    [0] {""}
   *[other] {" "}ve { $more } tane daha
}; önce bunları düzenleyin ya da kaldırın
op-data-rename-captured = “{ $reader }” sorgusu kendi tablolarından birine zaten “{ $name }” adını veriyor; o tabloyu sorgusunda yeniden adlandırın (set_query) ya da başka bir ad seçin
op-data-name-ambiguous = Birden çok veri “{ $name }” adını taşıyor ve sorgular bu adı okuyor, hangisi olduğu belirsiz; önce bu sorguları düzenleyin (set_query)
issue-unreadable-slide = okunamayan slayt, bir kenara ayrıldı
issue-unreadable-field = okunamayan sunum alanı { $field }, sıfırlandı
issue-knowledge-unreadable = okunamayan bilgi girdisi { $id }, bir kenara ayrıldı
issue-knowledge-entries-dropped = okunmadan atılan bilgi girdileri: { $count } ({ $cause ->
        [files] okunamayacak kadar çok dosya
        [file-bytes] okunamayacak kadar büyük dosya
        [total-bytes] okunamayacak kadar büyük paket
       *[unreadable] diğer okunamayan girdiler
    })
issue-knowledge-over-limit = bilgi koleksiyonu bir sınırı aşıyor: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (etkilenen dosyalar: { $count }, ilki { $id }); tüm dosyalar korunur; paketi yeniden sınırın altına yalnızca dosya silmek indirir (o zamana kadar yazmalar ve düzenlemeler reddedilir)
issue-knowledge-path-invalid = bilgi dosyası { $id } /recovered klasörüne taşındı: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = { $path } konumunda birden fazla bilgi dosyası vardı: diğerleri yeniden adlandırıldı
issue-knowledge-invalid = bilgi dosyası { $path } geçersiz: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
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
widget-fault-too-long = Widget { $widget }: bir ad, etiket, değer, açıklama veya liste sınırı aşıyor
issue-empty-item-id = { $param } listesinde tanımlayıcısı olmayan bir madde var
issue-duplicate-item-id = { $param } listesinde yinelenen bir madde tanımlayıcısı var
issue-invalid-item-id = { $param } listesinde eğik çizgi içeren bir öğe kimliği var
issue-list-bounds = { $param } listesinde çok az ya da çok fazla madde var
issue-view-truncated = { $element } verilerinin yalnızca bir kısmını gösteriyor ({ $what ->
    [rows] satırlar
   *[columns] sütunlar
}: { $total } içinden { $shown })
issue-render-budget = { $element } çizilmedi: sunumun görünümleri bir işlemenin izin verdiğinden fazla veri okuyor
issue-data-missing = { $element } sunumda olmayan “{ $data }” verisini adlandırıyor
issue-view-column-missing = { $element }, verisinde olmayan “{ $column }” sütununu okuyor
issue-view-unreadable = { $element } çizilmedi: seçenekleri okunamıyor
op-replica-full = Sunumun geçmişi neredeyse { $max } MiB'a ulaştı, yeniden açılabileceği en büyük boyut: daha fazla eklemeden önce içerik kaldırın.

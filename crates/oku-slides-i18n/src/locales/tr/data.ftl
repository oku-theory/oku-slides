data-cell-type = bir değer “{ $column }” sütununa uymuyor; bu sütun { $type ->
    [number] sayılar
    [integer] tam sayılar
    [bool] doğru veya yanlış
    [date] tarihler (YYYY-AA-GG)
   *[text] en fazla 16 KB metin
} içerir
data-column-type = “{ $column }” sütunu kayıpsız olarak { $type ->
    [number] sayı sütununa
    [integer] tam sayı sütununa
    [bool] doğru/yanlış sütununa
    [date] tarih sütununa
   *[text] metin sütununa
} dönüştürülemez, bkz. satırlar { $rows }{ $more ->
    [0] {""}
   *[other] {" "}ve { $more } satır daha
}
data-no-row = { $row } satırı yok
data-no-column = { $column } sütunu yok
data-id-taken = { $id } kimliği bu tabloda zaten kullanılıyor
data-id-invalid = “{ $id }” kimliği geçersiz: 1 ile 64 arası harf, rakam, - veya _ kullanın
data-id-missing = yeni bir satırın, sütunun veya tablonun kimliği yok
data-too-large = { $what ->
    [rows] veriler { $max } satırlık sınırını aşardı
    [columns] veriler { $max } sütunluk sınırını aşardı
    [text] bir metin hücresi { $max } baytlık sınırını aşardı
   *[overlay] veriler { $max } düzenlenmiş hücrelik sınırını aşardı
}
data-invalid-name = “{ $name }” geçerli bir veri adı değil: küçük harf, rakam ve _ kullanın, rakamla başlamayın
data-non-finite = { $count } sayı sonlu değil (NaN veya sonsuz)
data-columns-invalid = sütun listesi boş ya da bir sütunu iki kez adlandırıyor
data-row-width = bir satırda tablonun { $max } sütunundan daha fazla hücre var
data-needs-base = bu veriler satırlarını bir temel dosyada tutuyor; bu düzenleme onu henüz okuyamıyor
data-base-row-cells = temel dosyası olan bir tablo satırları için yalnızca kimlik verir, hücre vermez
data-row-cut = { $count } satırda başlıktan fazla hücre vardı; fazla hücreler kaldırıldı
data-base-invalid = verilerin temel dosyası bozuk veya tablosuyla eşleşmiyor
data-source-mismatch = verinin kaynağı türüyle uyuşmuyor: elle girilen veri bir tablo, sorgu verisi bir sorgu içerir
data-inline-query = sorgu verisi gömülü olamaz: gömülü veri her zaman bir tablodan gelir
data-query-parse = sorgu okunamıyor: satır { $line }, sütun { $column } konumunu kontrol edin
data-query-construct = sorgu “{ $construct }” kullanıyor; veri sorguları bunu desteklemiyor
data-query-too-long = sorgu { $max } bayttan uzun
data-query-too-deep = sorgu çok derin iç içe geçmiş
data-query-rewrite = sorgu motor için hazırlanamadı; bu bir Oku Slides hatası
data-query-unknown-function = sorgu “{ $function }” çağırıyor; veri sorguları bunu desteklemiyor
data-query-run = sorgu başarısız oldu: { $reason }
data-query-input-failed = “{ $input }” girdisi başarısız oldu
data-query-too-large = { $what ->
    [rows] sorgu sonucu { $max } satırlık sınırı aşıyor; sonuç asla kesilmez
    [columns] sorgu sonucu { $max } sütunluk sınırı aşıyor; sonuç asla kesilmez
   *[text] sorgu sonucundaki bir metin hücresi { $max } baytlık sınırı aşıyor; sonuç asla kesilmez
}
data-query-column-type = “{ $column }” sonuç sütununun türünü veriler tutamaz: onu metne, sayıya, tam sayıya, mantıksal değere veya tarihe dönüştürün
data-query-duplicate-column = iki sonuç sütununun adı “{ $column }”: birini AS ile yeniden adlandırın
data-query-float-operand = “{ $function }”, ondalık sayılar içeren “{ $operand }” üzerine uygulanamaz: önce onu tam sayıya dönüştürün
data-query-timeout = sorgu { $seconds } saniyeden uzun sürdü ve durduruldu

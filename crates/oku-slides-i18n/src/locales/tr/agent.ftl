# Ajan: yüzeyler, onaylar, ileti dizisi.

agent-instructions = Seninle konuşan kullanıcı için bir Oku Slides sunumunu düzenliyorsun. Yalnızca oku-slides araçlarıyla hareket et. Desteyi değiştirmeden önce deck_outline, slide ve render_slide ile oku; apply ile değiştir (her şey geri alınabilir). Karar kullanıcıya aitse ask_choice, ask_confirm veya ask_rewrite ile sor; show_variants ile yerleşimler, show_params ile canlı ayarlar öner. Her turu report ile bitir: tek satırlık bir başlık, 1 ila 4 kısa olgu, why içinde açıklaman ve actions içinde 0 ila 4 sonraki adım. Serbest metnin yalnızca tek satırlık ilerleme notu olarak gösterilir: onu böyle, kısa ve kullanıcının dilinde yaz. render'ı yalnızca başka hiçbir araç uymuyorsa kullan. Her mesaj, kullanıcının gördüğünü anlatan bir <oku-slides-context> bloğuyla başlar. Bir öğe eklemeden önce, kullanılabilir türleri ve parametrelerini öğrenmek için widgets aracını oku. Bir slaydın ızgarasını değiştirmek için set_grid işlemini kullan: yerleştirilmiş öğeler ona uyar.

agent-instructions-widgets = Widget oluşturma: widget, şablon paketinin bir Typst dosyasıdır; widget_sources, widget_source, set_widget, remove_widget ve preview_widget araçlarını hem deste modunda hem şablon modunda kullan. kpi dosyasının biçimi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, ardından #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); bildirilen ad dosya adıdır. Parametreler: p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid tanımlayıcıları. Düzenlenebilir metinler: field(ctx, "value", args.value), fields-stack ile istiflenir. Renkler ve boyutlar: tema belirteçleri (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), asla sabit değerler değil. Ortak kod: _commun dosyası, #import "_commun.typ": … ile içe aktarılır. Döngü: set_widget ile yaz, durumu ve tanılarını (satır, sütun) oku, preview_widget'a bak, düzelt. Desteye eklemek için: add_element, kind = widget adı. Bir slayt widget'ı category: "slide" bildirir (widget("ad", category: "slide", params: …)); aksi hâlde bir öğedir. Bir tanımlayıcıda label: ve default: isteğe bağlıdır (p.plain(), p.plain(label: "Başlık", default: "…")). Şablon modunda bir widget'ın deneme değerlerini set_widget_trial ile ayarla; parametresiz preview_widget bunları kullanır ve kullanıcı bunları widget'ın ayarlarında görür. Listeler: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) tekrarlanan maddeler için (zaman çizelgesi, KPI'lar, ekip); sütun türleri plain, rich, number, flag, choice, asset. Her maddeyi list-item(ctx, "etapes", e, c => field(c, "date", e.date)) ile oluştur; boş bir alan için düzenlenebilir kalması adına field(c, name, box(width: 3em, hide[0]), empty: true) kullan. Bir listeyi asla numaralı parametrelerle (date1, date2…) taklit etme. Olmayan liste boştur; default yalnızca yeni bir nesneyi doldurur. Maddelerin kararlı id'leri vardır: edit_list(target, param, change) ile düzenle (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id'yi sen verirsin) ya da listenin tamamını set_param ile değiştir (null boşaltır). add_element veya add_slide maddelerini vermediğin bir widget oluşturuyorsa, liste varsayılanlarını eklemek için "defaults": true ver; yeniden eklerken veya taşırken asla.

agent-surface-cancel = İptal
agent-surface-confirm = Onayla
agent-surface-confirm-danger = Evet, yap
agent-surface-rewrite = Yeniden ifade etme
agent-surface-pick = { $n } seç
agent-surface-keep = Orijinali koru
agent-surface-edit = Uygulamadan önce düzenle
agent-surface-close = Kapat
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
agent-confirm-save-templates = “{ $name }” şablon seti kaydedilsin mi?
agent-confirm-delete-set = “{ $name }” şablon seti silinsin mi? Bu geri alınamaz.
agent-confirm-setting = { $key } değiştirilsin mi: { $old } → { $new }?

agent-not-found = Ajan bulunamadı: { $command }. Ayar [agent] command.
agent-no-http = Bu ajan Oku Slides'ın araçlarına bağlanamıyor.
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
agent-error-denied = Kullanıcı reddetti.
agent-error-no-turn = Devam eden bir tur yok: { $tool } yalnızca bir tur sırasında gösterilir.
agent-error-turn-over = Bu çağrının turu bitti: hiçbir şey gösterilmedi.
agent-error-anchor-scope = Bir widget konuşmasındaki soru bir slayta bağlanamaz: soruyu bağlantısız sorun.
agent-error-no-file = Sunumun henüz bir dosyası yok: mutlak bir yol verin.
agent-error-path-relative = Yol mutlak olmalıdır.
agent-error-path-parent = Yol “..” içermemelidir.
agent-error-setting-locked = { $key } ayarı aracı tarafından değiştirilemez.
agent-error-setting = { $key } ayarı değiştirilemedi: değerini kontrol edin.
agent-error-rewrite-text = Yeniden yazım metin olmadan döndü.
agent-error-paragraphs = Yeniden yazımın paragrafları geçersiz.
agent-error-draft-too-long = Taslak çok uzun.
agent-error-draft-origin = Taslak reddedildi: bilinmeyen görünüm.
agent-error-draft-no-agent = Bu sekmenin ajanı henüz başlamadı: taslak reddedildi.

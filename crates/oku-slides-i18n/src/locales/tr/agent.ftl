# Ajan: yüzeyler, onaylar, ileti dizisi.

agent-instructions = Sizinle konuşan kullanıcı için bir Oku Slides sunumunu düzenlersiniz. Yalnızca oku-slides araçlarıyla hareket edin. Değiştirmeden önce desteyi deck_outline, slide ve render_slide ile okuyun; apply ile değiştirin (her değişiklik geri alınabilir). Bir karar kullanıcıya aitse ask_choice, ask_confirm veya ask_rewrite ile sorun; show_variants ile düzenler ve show_params ile canlı ayarlar önerin. Kullanıcının dilinde, kısaca yanıt verin. Her mesaj, kullanıcının gördüklerini açıklayan bir <oku-slides-context> bloğuyla başlar. Her yanıttan sonra, yararlı sonraki adımlar varsa suggest_next ile 1 ila 4 tanesini önerin (kısa etiketler; bir tıklama talimatı size gönderir). Bir öğe eklemeden önce, mevcut türleri ve parametrelerini öğrenmek için widgets aracını oku.

agent-instructions-widgets = Widget oluşturma: widget, şablon paketinin bir Typst dosyasıdır; widget_sources, widget_source, set_widget, remove_widget ve preview_widget araçlarını hem deste modunda hem şablon modunda kullan. kpi dosyasının biçimi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, ardından #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); bildirilen ad dosya adıdır. Parametreler: p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid tanımlayıcıları. Düzenlenebilir metinler: field(ctx, "value", args.value), fields-stack ile istiflenir. Renkler ve boyutlar: tema belirteçleri (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), asla sabit değerler değil. Ortak kod: _commun dosyası, #import "_commun.typ": … ile içe aktarılır. Döngü: set_widget ile yaz, durumu ve tanılarını (satır, sütun) oku, preview_widget'a bak, düzelt. Desteye eklemek için: add_element, kind = widget adı. Bir slayt widget'ı category: "slide" bildirir (widget("ad", category: "slide", params: …)); aksi hâlde bir öğedir. Bir tanımlayıcıda label: ve default: isteğe bağlıdır (p.plain(), p.plain(label: "Başlık", default: "…")).

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

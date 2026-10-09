# एजेंट: सतहें, पुष्टिकरण, थ्रेड।

agent-instructions = आप उपयोगकर्ता के लिए एक Oku Slides प्रस्तुति संपादित करते हैं, जो आपसे बात करता है। केवल oku-slides टूल के माध्यम से काम करें। डेक बदलने से पहले उसे deck_outline, slide और render_slide से पढ़ें; उसे apply से बदलें (सब कुछ पूर्ववत किया जा सकता है)। जब कोई निर्णय उपयोगकर्ता का हो, तो ask_choice, ask_confirm या ask_rewrite से पूछें; show_variants से लेआउट और show_params से लाइव सेटिंग सुझाएँ। हर बारी report से समाप्त करें: एक पंक्ति का शीर्षक, 1 से 4 छोटे तथ्य, why में आपकी व्याख्या, और actions में 0 से 4 अगले कदम। आपका मुक्त पाठ केवल एक पंक्ति के प्रगति नोट के रूप में दिखाया जाता है: उसे वैसे ही, संक्षेप में और उपयोगकर्ता की भाषा में लिखें। render का उपयोग केवल तब करें जब कोई और टूल उपयुक्त न हो। हर संदेश <oku-slides-context> ब्लॉक से शुरू होता है, जो बताता है कि उपयोगकर्ता क्या देख रहा है। कोई तत्व जोड़ने से पहले, उपलब्ध प्रकार और उनके पैरामीटर जानने के लिए widgets टूल पढ़ें। किसी स्लाइड की ग्रिड बदलने के लिए set_grid op का उपयोग करें: रखे गए तत्व साथ चलते हैं।

agent-instructions-widgets = विजेट बनाना: विजेट टेम्पलेट पैक की एक Typst फ़ाइल है; डेक मोड और टेम्पलेट मोड, दोनों में widget_sources, widget_source, set_widget, remove_widget और preview_widget का उपयोग करो। kpi फ़ाइल का रूप: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, फिर #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); घोषित नाम ही फ़ाइल का नाम है। पैरामीटर: डिस्क्रिप्टर p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid। संपादन योग्य पाठ: field(ctx, "value", args.value), fields-stack से ऊपर-नीचे रखे जाते हैं। रंग और आकार: थीम के टोकन (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), कभी तयशुदा मान नहीं। साझा कोड: _commun फ़ाइल, जिसे #import "_commun.typ": … से आयात किया जाता है। चक्र: set_widget से लिखो, स्थिति और उसके निदान (पंक्ति, स्तंभ) पढ़ो, preview_widget देखो, सुधारो। इसे डेक में डालने के लिए: add_element, जिसमें kind विजेट का नाम हो। स्लाइड विजेट category: "slide" घोषित करता है (widget("name", category: "slide", params: …)); अन्यथा वह एक तत्व है। डिस्क्रिप्टर में label: और default: वैकल्पिक हैं (p.plain(), p.plain(label: "Title", default: "…"))। टेम्पलेट मोड में set_widget_trial से किसी विजेट के परीक्षण मान सेट करो; बिना पैरामीटर के preview_widget उन्हें इस्तेमाल करता है, और उपयोगकर्ता उन्हें विजेट की सेटिंग में देखता है। सूचियाँ: दोहराए जाने वाले आइटम (टाइमलाइन, KPI, टीम) के लिए p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8); कॉलम प्रकार plain, rich, number, flag, choice, asset। हर आइटम को list-item(ctx, "etapes", e, c => field(c, "date", e.date)) से रेंडर करें; खाली फ़ील्ड के लिए field(c, name, box(width: 3em, hide[0]), empty: true) का उपयोग करें ताकि वह संपादन योग्य रहे। सूची को क्रमांकित पैरामीटर (date1, date2…) से कभी न बनाएँ। अनुपस्थित सूची खाली होती है; default केवल नए ऑब्जेक्ट को भरता है। आइटम की id स्थिर होती हैं: उन्हें edit_list(target, param, change) से संपादित करें (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id आप देते हैं), या पूरी सूची set_param से बदलें (null उसे खाली करता है)। जब add_element या add_slide आपके आइटम दिए बिना विजेट बनाए, तो सूची के डिफ़ॉल्ट भरने के लिए "defaults": true दें; दोबारा डालते या खिसकाते समय कभी नहीं।

agent-surface-cancel = रद्द करें
agent-surface-confirm = पुष्टि करें
agent-surface-confirm-danger = हाँ, करें
agent-surface-rewrite = पुनर्लेखन
agent-surface-pick = { $n } चुनें
agent-surface-keep = मूल रखें
agent-surface-edit = लागू करने से पहले संपादित करें
agent-surface-close = बंद करें
agent-surface-variants = वैरिएंट
agent-surface-permission-always = हमेशा के लिए
agent-surface-permission-title = एजेंट एक अनुमति माँग रहा है
agent-undo-turn = एजेंट: { $text }

agent-tools-unavailable = एजेंट के टूल शुरू नहीं हो सके।

agent-undo-apply = एजेंट परिवर्तन
agent-undo-variant = वैरिएंट: { $label }
agent-undo-param = सेटिंग: { $label }
agent-undo-rewrite = पुनर्लेखन

agent-confirm-save = प्रस्तुति को { $path } के रूप में सहेजें?
agent-confirm-save-over = { $path } को बदलें? मौजूदा फ़ाइल अधिलेखित हो जाएगी।
agent-confirm-open = { $path } खोलें?
agent-open-elsewhere = यह फ़ाइल पहले से किसी दूसरे टैब में खुली है।
agent-confirm-new = नई प्रस्तुति शुरू करें?
agent-confirm-new-dirty = नई प्रस्तुति शुरू करें? न सहेजे गए बदलाव खो जाएँगे।
agent-confirm-save-templates = टेम्पलेट सेट “{ $name }” सहेजें?
agent-confirm-delete-set = टेम्पलेट सेट “{ $name }” हटाएं? यह पूर्ववत नहीं किया जा सकता।
agent-confirm-setting = { $key } बदलें: { $old } → { $new }?

agent-not-found = एजेंट नहीं मिला: { $command }। सेटिंग [agent] command।
agent-no-http = यह एजेंट Oku Slides के टूल से कनेक्ट नहीं हो सकता।
agent-stopped = एजेंट रुक गया।
agent-failed = एजेंट ने अनुरोध अस्वीकार कर दिया: { $message }
agent-tool-failed = टूल अप्रत्याशित रूप से विफल हो गया।

agent-turn-tools = { $tools ->
   [one] { $tools } टूल
  *[other] { $tools } टूल
}
agent-turn-notes = { $notes ->
   [one] { $notes } नोट
  *[other] { $notes } नोट
}
agent-turn-secs = { $secs } से.
agent-turn-stopped = रोका गया।
agent-turn-interrupted = बाधित किया गया।
agent-memory-lost = एजेंट को यह बातचीत अब याद नहीं है: वह फिर से शुरू करता है।
agent-turn-max-tokens = एजेंट अपनी लंबाई सीमा तक पहुँच गया।
agent-turn-max-turns = एजेंट अपनी चरण सीमा तक पहुँच गया।
agent-turn-refusal = एजेंट ने मना कर दिया।
agent-turn-ended = बारी समाप्त ({ $stop })।
agent-choice-cancelled = प्रश्न रद्द किया गया
agent-choice-answered = ✓ { $answer }
agent-report-why = क्यों

agent-error-file = फ़ाइल पढ़ी या लिखी नहीं जा सकी।
agent-error-unexpected-action = अनपेक्षित क्रिया: { $action }।
agent-error-unknown-surface = यह कार्ड अब खुला नहीं है।
agent-error-expired = पिछले टर्न की क्रियाओं की समय-सीमा समाप्त हो गई है।
agent-error-incomplete = इस कार्ड की क्रिया अधूरी है।
agent-error-no-field = यह सेटिंग अब मौजूद नहीं है।
agent-error-no-variant = यह वैरिएंट अब मौजूद नहीं है।
agent-error-arguments = टूल के आर्ग्युमेंट अमान्य हैं ({ $detail }): उन्हें उसके स्कीमा से जाँचें।
agent-error-unknown-tool = अज्ञात टूल: { $name }।
agent-error-denied = उपयोगकर्ता ने मना कर दिया।
agent-error-no-turn = कोई टर्न चालू नहीं है: { $tool } केवल टर्न के दौरान दिखाया जाता है।
agent-error-turn-over = इस कॉल का टर्न समाप्त हो चुका है: कुछ नहीं दिखाया गया।
agent-error-anchor-scope = विजेट वार्तालाप का प्रश्न किसी स्लाइड से नहीं जोड़ा जा सकता: इसे बिना एंकर के पूछें।
agent-error-no-file = प्रस्तुति की अभी कोई फ़ाइल नहीं है: पूर्ण पथ दें।
agent-error-path-relative = पथ पूर्ण होना चाहिए।
agent-error-path-parent = पथ में “..” नहीं होना चाहिए।
agent-error-setting-locked = एजेंट सेटिंग { $key } नहीं बदल सकता।
agent-error-setting = सेटिंग { $key } नहीं बदली जा सकी: उसका मान जाँचें।
agent-error-rewrite-text = पुनर्लेखन बिना पाठ के लौटा।
agent-error-paragraphs = पुनर्लेखन के अनुच्छेद अमान्य हैं।
agent-error-draft-too-long = ड्राफ़्ट बहुत लंबा है।
agent-error-draft-origin = ड्राफ़्ट अस्वीकृत: अज्ञात दृश्य।
agent-error-draft-no-agent = इस टैब का एजेंट शुरू हो रहा है: ड्राफ़्ट सहेजा नहीं गया।

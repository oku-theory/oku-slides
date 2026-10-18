# ऐप का इंटरफ़ेस (React)। ui/src/i18n.ts इसे «messages» अनुरोध के ज़रिए लोड करता है।
ui-loading = लोड हो रहा है…

## दस्तावेज़

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · टेम्पलेट
ui-title-rename = प्रेज़ेंटेशन का नाम बदलें
ui-title-field = प्रेज़ेंटेशन का शीर्षक
ui-status-unsaved = बिना सहेजे बदलाव
ui-status-saved = सहेजा गया
ui-status-never-saved = अभी सहेजा नहीं गया
ui-caption-templates = { $count ->
    [one] { $count } टेम्पलेट
   *[other] { $count } टेम्पलेट
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } स्लाइड
   *[other] { $count } स्लाइड
} · { $status }
ui-template-default-name = टेम्पलेट
ui-template-variant-name = { $name } (वैरिएंट)
ui-confirm-quit-templates = बदले गए टेम्पलेट सहेजे बिना बाहर निकलें?

## सूचनाएँ

ui-toast-templates-saved = टेम्पलेट सहेजे गए: { $name }
ui-toast-moved = प्रस्तुति ले जाई गई
ui-toast-copied = प्रति सहेजी गई
ui-toast-field-gone = आप जिस फ़ील्ड को संपादित कर रहे थे वह अब मौजूद नहीं है: आपका लिखा खो गया।
ui-toast-edit-replaced = आपके संपादन के दौरान यह टेक्स्ट बदल गया: आपके संस्करण ने उसकी जगह ले ली। पूर्ववत करें दूसरा संस्करण वापस लाता है।

## टूलबार

ui-toolbar-undo = पूर्ववत करें
ui-toolbar-undo-label = पूर्ववत करें: { $label }
ui-toolbar-redo = फिर से करें
ui-toolbar-redo-label = फिर से करें: { $label }
ui-toolbar-grid-hide = ग्रिड छिपाएँ
ui-toolbar-grid-show = ग्रिड दिखाएँ
ui-toolbar-ghosts-hide = गाइड छिपाएँ
ui-toolbar-ghosts-show = गाइड दिखाएँ
ui-divider-pile = स्लाइड ढेर की चौड़ाई
ui-divider-notes = एजेंट थ्रेड की चौड़ाई
ui-divider-code = कोड का आकार
ui-divider-band = सेटिंग्स की ऊँचाई
ui-toolbar-theme = थीम
ui-toolbar-theme-embedded = फ़ाइल की थीम
ui-toolbar-save-copy = कॉपी सहेजें
ui-toolbar-finish = हो गया
ui-menu-workshop = कार्यशाला
ui-menu-workshop-deck-only = ये टेम्पलेट केवल इसी प्रस्तुति के हैं; कार्यशाला केवल लाइब्रेरी के टेम्पलेट संपादित करती है।
ui-toolbar-open = खोलें
ui-merge = PDF के साथ मिलाएँ…
ui-menu = मेनू
ui-menu-move = ले जाएँ…
ui-language = भाषा
ui-language-system = सिस्टम की भाषा
ui-scheme = रूप
ui-scheme-system = सिस्टम
ui-scheme-light = हल्का
ui-scheme-dark = गहरा
ui-toolbar-add-label = स्लाइड में जोड़ें
ui-add-kind = { $kind ->
    [card] कार्ड
    [shape] आकृति
    [chart] चार्ट
    [table] तालिका
   *[text] टेक्स्ट
}
ui-add-widget = पैक विजेट
ui-toolbar-remove-template = टेम्पलेट हटाएँ
ui-toolbar-remove-slide = स्लाइड हटाएँ
ui-toolbar-remove-hint = स्लाइड { $n } · ⌘Z से पूर्ववत करें

## पूर्ववत लेबल

ui-undo-add = { $kind ->
    [card] कार्ड जोड़ें
    [shape] आकृति जोड़ें
    [chart] चार्ट जोड़ें
    [table] तालिका जोड़ें
   *[text] टेक्स्ट जोड़ें
}
ui-undo-add-widget = “{ $name }” जोड़ें
ui-undo-new-template = नया टेम्पलेट: { $name }
ui-undo-new-slide = नई स्लाइड: { $name }
ui-undo-templates = टेम्पलेट: { $name }
ui-undo-move-template = टेम्पलेट खिसकाएँ
ui-undo-move-slide = स्लाइड खिसकाएँ
ui-undo-remove-slide = स्लाइड हटाएँ
ui-undo-remove = हटाएँ
ui-undo-paste = चिपकाएँ
ui-undo-place-on-grid = ग्रिड पर रखें
ui-undo-move-on-grid = ग्रिड पर खिसकाएँ
ui-undo-resize-on-grid = ग्रिड पर आकार बदलें
ui-undo-change-anchor = एंकरिंग बदलें
ui-undo-move = खिसकाएँ
ui-undo-resize = आकार बदलें
ui-undo-edit-slide = स्लाइड संपादित करें
ui-undo-rename-set = सेट का नाम बदलें
ui-undo-rename-deck = प्रेज़ेंटेशन का नाम बदलें
ui-undo-chart-kind = चार्ट का प्रकार बदलें
ui-undo-chart-data = डेटा संपादित करें
ui-undo-chart-color = श्रृंखला को रंगें
ui-undo-chart-stack = { $stacked ->
    [yes] श्रृंखलाओं को स्टैक करें
   *[no] श्रृंखलाओं को अलग करें
}

## नए एलिमेंट

ui-new-text = नया टेक्स्ट
ui-new-card-title = कार्ड का शीर्षक
ui-new-card-detail = विवरण

## स्लाइड सूची

ui-pile-slides = स्लाइड
ui-pile-templates = टेम्पलेट
ui-pile-add-slide = + नई स्लाइड
ui-pile-add-template = + नया टेम्पलेट
ui-pile-drag-hint = खिसकाने के लिए खींचें · ⌥↑ ⌥↓
ui-slide-alt = स्लाइड { $n }

## स्टेज

ui-stage-rendering = रेंडर हो रहा है…
ui-stage-elements = स्लाइड के एलिमेंट
ui-stage-fields = स्लाइड के फ़ील्ड
ui-stage-edit-text = टेक्स्ट संपादित करें
ui-stage-hint-plain = अगले पर जाने के लिए Enter या Tab · रद्द करने के लिए Esc
ui-stage-hint-rich = अगले पर जाने के लिए Tab · पुष्टि के लिए ⌘ Enter · बुलेट के लिए “- ” · ⌘B बोल्ड, ⌘I इटैलिक, ⌘K लिंक
ui-field = { $kind ->
    [title] शीर्षक
    [subtitle] उपशीर्षक
    [kicker] किकर
    [text] टेक्स्ट
    [body] मुख्य भाग
    [author] लेखक
    [date] तारीख
    [organization] संगठन
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] टेक्स्ट
    [card] कार्ड
    [image] इमेज
    [shape] आकृति
    [group] समूह
   *[other] { $kind }
}
ui-grid-cell = कॉलम { $cols } · पंक्ति { $rows }
ui-ghost-zone = क्षेत्र

## स्लाइड नोट्स

ui-notes = स्लाइड टूल
ui-stepper-less = { $label }: एक कम
ui-stepper-more = { $label }: एक और
ui-list-item = आइटम
ui-list-add = { $item } जोड़ें
ui-list-remove = { $item } हटाएँ
ui-list-move = { $item } खिसकाएँ
ui-list-full = सूची भरी हुई है ({ $max })
ui-undo-list-move = { $item } खिसकाएँ
ui-agent-color-other = अन्य रंग
ui-agent-thread = एजेंट
ui-agent-new-conversation = नई बातचीत
ui-agent-undo-turn = इस बारी को पूर्ववत करें
ui-agent-detach = एजेंट को अलग विंडो में खोलें
ui-agent-attach = एजेंट को मुख्य विंडो में वापस लाएँ
ui-agent-window-empty = इस टैब के लिए कोई एजेंट नहीं है
ui-agent-kept-in-draft = लिखने की जगह नहीं: पाठ ड्राफ़्ट में रखा गया
ui-agent-busy-elsewhere = एजेंट { $place } में उत्तर दे रहा है…
ui-agent-scope-slides = स्लाइड
ui-agent-scope-atelier = कार्यशाला
ui-draft = मसौदा
ui-draft-send = एजेंट को भेजें
ui-draft-send-title = एजेंट को भेजें (⌘↩)
ui-draft-discard = हटाएँ
ui-agent-state-idle = एजेंट
ui-agent-state-starting = एजेंट शुरू हो रहा है…
ui-agent-state-working = एजेंट काम कर रहा है…
ui-agent-dock = जारी है
ui-agent-dock-settings = सेटिंग
ui-agent-jump = ↓ मौजूदा बारी
ui-agent-tool-input = तर्क
ui-agent-tool-output = परिणाम
ui-agent-sheet = एजेंट का प्रश्न
ui-agent-sheet-keys = 1–4 चुनें · Enter पुष्टि · E संपादित करें · Esc रद्द करें
ui-agent-variants = विकल्प
ui-agent-drawer-keys = क्लिक या 1–4 · Esc बंद करता है
ui-agent-confirm = एजेंट की पुष्टि

## डिक्टेशन

ui-dictation = डिक्टेशन
ui-dictation-start = बोलकर लिखें
ui-dictation-start-title = बोलकर लिखें (⌘D)
ui-dictation-type = एजेंट को लिखें
ui-dictation-type-title = एजेंट को लिखें (⌘K)
ui-dictation-agent-stop = एजेंट रोकें (⌘⇧D या Esc)
ui-dictation-agent-working = एजेंट काम कर रहा है…
ui-dictation-agent-working-hint = ⌘⇧D या Esc उसे रोकता है
ui-dictation-finish = डिक्टेशन समाप्त करें
ui-dictation-finish-title = समाप्त करें (⌘D)
ui-dictation-cancel = रद्द करें
ui-dictation-preparing = डिक्टेशन तैयार हो रहा है…
ui-dictation-heard-nothing = कुछ सुनाई नहीं दिया: माइक्रोफ़ोन के थोड़ा पास बोलें
ui-dictation-to-cursor = कर्सर पर
ui-dictation-to-draft = मसौदे में
ui-dictation-downloading = { $name } मॉडल डाउनलोड हो रहा है
ui-dictation-download-progress = { $percent }% · { $total } में से { $received } · सिर्फ़ एक बार
ui-dictation-listening = सुन रहा है · { $time }
ui-dictation-transcribing = आख़िरी वाक्य लिखा जा रहा है…
ui-dictation-stop-in = { $duration } में रुकेगा
ui-dictation-speak = बोलिए, टेक्स्ट यहाँ दिखेगा…
ui-dictation-auto-stop = { $duration } की चुप्पी के बाद रुकता है
ui-dictation-to-download = { $size } डाउनलोड करना है
ui-dictation-settings = सेटिंग
ui-dictation-model-title = पहचान मॉडल
ui-dictation-model-intro = Whisper, इसी कंप्यूटर पर (CTranslate2)। हर मॉडल कई भाषाएँ समझता है।
ui-dictation-note-turbo = अनुशंसित, तेज़ और सटीक
ui-dictation-note-large = सबसे सटीक, धीमा
ui-dictation-note-base = हल्का, छोटी मशीनों के लिए
ui-dictation-installed = इंस्टॉल है
ui-dictation-not-installed = डाउनलोड करना है
ui-dictation-language = बोली जाने वाली भाषा
ui-dictation-language-auto = स्वचालित पहचान
ui-dictation-note-apple = सबसे तेज़, सिस्टम में शामिल
ui-dictation-model-unavailable = भाषा समर्थित नहीं है
ui-dictation-model-intro-apple = Apple, सिस्टम में शामिल, या इसी कंप्यूटर पर Whisper (CTranslate2)। सभी कई भाषाएँ समझते हैं।
ui-dictation-language-auto-resolved = स्वचालित ({ $language })
ui-dictation-to-download-language = भाषा डाउनलोड करनी होगी
ui-dictation-auto-stop-label = इतनी देर की चुप्पी के बाद अपने आप रुकें
ui-dictation-silence = चुप्पी की अवधि
ui-dictation-phrases-note = आपके बोलते समय केवल बोले गए अंश ही, वाक्य-दर-वाक्य, लिखे जाते हैं।
ui-dictation-download-now = अभी डाउनलोड करें ({ $size })
ui-dictation-download-language = भाषा डाउनलोड करें
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } सेकंड
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## टेम्पलेट चयनकर्ता

ui-picker-new-slide = नई स्लाइड
ui-picker-new-template = नया टेम्पलेट, इस पर आधारित…
ui-picker-set = टेम्पलेट
ui-picker-set-mine = { $name } · मेरे
ui-picker-set-received = { $name } · प्राप्त
ui-picker-keys = ब्राउज़ करने के लिए ← → · जोड़ने के लिए Enter · बंद करने के लिए Esc
ui-picker-close = बंद करें
ui-picker-previous = पिछला टेम्पलेट
ui-picker-next = अगला टेम्पलेट
ui-picker-list = स्लाइड टेम्पलेट

## चार्ट

ui-chart-panel = चार्ट
ui-chart-kinds = चार्ट का प्रकार
ui-chart-kind = { $kind ->
    [line] लाइनें
    [pie] पाई
    [scatter] बिंदु
    [radar] रडार
    [waterfall] वॉटरफ़ॉल
   *[bar] बार
}
ui-chart-no-data = इस चार्ट में कोई डेटा तालिका नहीं है।
ui-chart-options = चार्ट विकल्प
ui-chart-stack = स्टैक्ड
ui-chart-waterfall-hint = कुल बार के लिए «कुल» पर टिक करें।
ui-chart-colors = श्रृंखलाओं के रंग
ui-chart-color-of = { $name } का रंग
ui-chart-color-auto = स्वचालित
ui-chart-color-pick = रंग { $n }
ui-chart-column-name = कॉलम { $col } का नाम
ui-chart-remove-column = कॉलम हटाएँ
ui-chart-cell = पंक्ति { $row }, कॉलम { $col }
ui-chart-remove-row = पंक्ति हटाएँ
ui-chart-add-row = पंक्ति
ui-chart-add-column = कॉलम
ui-chart-series = सीरीज़ { $n }

## तालिका

ui-table-panel = तालिका
ui-table-options = तालिका के विकल्प
ui-table-header-row = शीर्ष पंक्ति
ui-table-header-column = पहला स्तंभ शीर्षक के रूप में
ui-table-striped = धारीदार पंक्तियाँ
ui-table-alignment = स्तंभों का संरेखण
ui-table-align-of = { $name } का संरेखण
ui-table-align = { $align ->
    [start] आरंभ
    [center] केंद्र
    [end] अंत
   *[auto] स्वचालित
}
ui-table-column = स्तंभ { $n }
ui-table-no-data = इस तालिका में कोई डेटा नहीं है।
ui-undo-table-option = तालिका के विकल्प
ui-undo-table-align = स्तंभ संरेखित करें
ui-undo-table-data = तालिका संपादित करें
ui-show-as = इस रूप में दिखाएँ
ui-show-as-table = तालिका के रूप में दिखाएँ
ui-show-as-chart = चार्ट के रूप में दिखाएँ
ui-undo-show-as-table = तालिका के रूप में दिखाएँ
ui-undo-show-as-chart = चार्ट के रूप में दिखाएँ

## फ़ाइलें

ui-file-open-prompt = खोलने के लिए फ़ाइल का पाथ (.oku-slides.pdf, .json या .pptx)
ui-file-merge-prompt = इस प्रस्तुति में मिलाने के लिए फ़ाइल का पाथ (.oku-slides.pdf या .json)
ui-file-filter-presentations = प्रस्तुतियाँ
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = इस रूप में सहेजें (पूरा पाथ)

## टैब

ui-tabs = खुली प्रस्तुतियाँ
ui-tab-new = नया टैब
ui-tab-close = टैब बंद करें
ui-tab-unsaved = बिना सहेजे बदलाव
ui-tab-templates = टेम्पलेट
ui-tab-move-new-window = नई विंडो में ले जाएँ
ui-tab-move-to = विंडो { $n } में ले जाएँ
ui-tab-close-others = अन्य टैब बंद करें
ui-close-cancel = रद्द करें
ui-disk-replaced-question = डिस्क पर { $file } को किसी दूसरी प्रस्तुति से बदल दिया गया है।
ui-disk-replaced-overwrite = अधिलेखित करें
ui-disk-replaced-save-as = इस रूप में सहेजें…
ui-same-deck-question = { $file } टैब “{ $title }” में खुली प्रस्तुति का एक संस्करण है।
ui-same-deck-merge = उस टैब में मिलाएँ
ui-same-deck-apart = अलग से खोलें
ui-history = इतिहास
ui-history-version = { $date } का संस्करण
ui-history-restore = पुनर्स्थापित करें
ui-history-back = वापस
ui-history-anonymous = अनाम
ui-history-slider = प्रस्तुति के संस्करण
ui-history-removed-since = तब से हटाई गई
ui-history-changed-since = तब से बदली गई
ui-history-added-since = तब से जोड़ी गई स्लाइडें: { $count }
ui-help = सहायता
ui-help-caption = स्लाइड { $index } / { $count } · केवल पढ़ने के लिए
ui-slide-previous = पिछली स्लाइड
ui-slide-next = अगली स्लाइड
ui-present = प्रस्तुति
ui-present-slide = स्लाइड { $index } / { $count }
ui-present-screen = पूर्ण स्क्रीन में प्रस्तुत करें
ui-present-window = विंडो में प्रस्तुत करें
ui-toolbar-params-hide = सेटिंग्स छिपाएँ
ui-toolbar-params-show = सेटिंग्स दिखाएँ
ui-help-close = सहायता बंद करें
ui-help-search = सहायता में खोजें
ui-help-search-placeholder = खोजें…
ui-help-search-empty = कोई परिणाम नहीं
ui-help-search-count = { $count } परिणाम
ui-pack-notice-close = बंद करें
ui-widgets-files = विजेट फ़ाइलें
ui-widgets-common = साझा
ui-widgets-new = नया विजेट
ui-widgets-name = विजेट का नाम (छोटे अक्षर, अंक और हाइफ़न)
ui-widgets-name-taken = यह नाम पहले से लिया गया है
ui-widgets-name-letter = नाम किसी अक्षर से शुरू होना चाहिए
ui-widgets-valid = मान्य
ui-widgets-errors = त्रुटियाँ
ui-widgets-empty = इस पैक में अभी कोई विजेट नहीं है।
ui-widgets-preview = विजेट पूर्वावलोकन
ui-widgets-no-preview = कोई पूर्वावलोकन नहीं
ui-widgets-field-bound = प्रस्तुति से जुड़ा: यहाँ केवल पढ़ने के लिए
ui-widgets-source = स्रोत कोड
ui-widgets-unsaved = सहेजा नहीं गया
ui-widgets-in-error = त्रुटि वाले विजेट
ui-widgets-none-in-error = किसी विजेट में त्रुटि नहीं है
ui-widgets-section = विजेट
ui-widgets-open = विजेट { $name } खोलें
ui-widgets-delete-named = { $name } हटाएँ
ui-widgets-confirm-delete = विजेट “{ $name }” हटाएँ?
ui-widgets-rename-hint = नाम बदलने के लिए डबल-क्लिक करें या F2 दबाएँ
ui-widgets-tabs = टेम्पलेट टैब
ui-widgets-templates-tab = टेम्पलेट
ui-theme-tab = थीम
ui-theme-picker = थीम
ui-theme-name = थीम का नाम
ui-theme-unsaved = थीम के बदलाव सहेजे नहीं गए
ui-widgets-close-tab = { $name } बंद करें
ui-widgets-removed-by-agent = एजेंट ने विजेट { $name } हटा दिया।

## आयात करें

ui-import-title = PowerPoint प्रस्तुति आयात करें
ui-import-mode = आयात मोड
ui-import-hifi = हूबहू प्रति
ui-import-hifi-note = इस प्रस्तुति के लिए बनी थीम और टेम्पलेट के साथ मूल का रूप बनाए रखता है।
ui-import-retrofit = मेरे टेम्पलेट इस्तेमाल करें
ui-import-retrofit-note = लाइब्रेरी के टेम्पलेट और एक थीम से स्लाइड फिर से बनाता है।
ui-import-theme-default = डिफ़ॉल्ट थीम
ui-import-no-packs = लाइब्रेरी में कोई टेम्पलेट सेट नहीं है।
ui-import-tune = एजेंट के साथ बारीकी से सुधारें
ui-import-tune-note = एजेंट हर स्लाइड की मूल से तुलना करता है और अंतर ठीक करता है।
ui-import-tune-note-retrofit = एजेंट हर स्लाइड की मूल से तुलना करता है और उन आकृतियों के लिए विजेट बनाता है जिन्हें आपके टेम्पलेट नहीं दिखा सकते।
ui-import-submit = आयात करें
ui-import-busy = आयात हो रहा है…

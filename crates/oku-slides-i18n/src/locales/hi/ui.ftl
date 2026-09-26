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
ui-status-builtin = बिल्ट-इन: सहेजने पर एक कॉपी बनेगी
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

ui-toast-saved = आपका प्रेज़ेंटेशन सहेज लिया गया
ui-toast-templates-saved = टेम्पलेट सहेजे गए: { $name }
ui-toast-select-zone = रखने के लिए एक क्षेत्र चुनें: “{ $text }”

## टूलबार

ui-toolbar-undo = पूर्ववत करें
ui-toolbar-undo-label = पूर्ववत करें: { $label }
ui-toolbar-redo = फिर से करें
ui-toolbar-redo-label = फिर से करें: { $label }
ui-toolbar-grid-hide = ग्रिड छिपाएँ
ui-toolbar-grid-show = ग्रिड दिखाएँ
ui-toolbar-grid-hide-title = ग्रिड छिपाएँ (⌘')
ui-toolbar-grid-show-title = ग्रिड दिखाएँ (⌘')
ui-toolbar-theme = थीम
ui-toolbar-theme-default = डिफ़ॉल्ट
ui-toolbar-save-copy = कॉपी सहेजें
ui-toolbar-save = सहेजें
ui-toolbar-finish = हो गया
ui-toolbar-templates = टेम्पलेट
ui-toolbar-templates-title = स्लाइड टेम्पलेट बनाएँ और संपादित करें
ui-toolbar-open = खोलें
ui-toolbar-add-label = स्लाइड में जोड़ें
ui-toolbar-add = जोड़ें
ui-add-kind = { $kind ->
    [card] कार्ड
    [shape] आकृति
    [chart] चार्ट
   *[text] टेक्स्ट
}
ui-toolbar-remove-template = टेम्पलेट हटाएँ
ui-toolbar-remove-slide = स्लाइड हटाएँ
ui-toolbar-remove-hint = स्लाइड { $n } · ⌘Z से पूर्ववत करें

## पूर्ववत लेबल

ui-undo-add = { $kind ->
    [card] कार्ड जोड़ें
    [shape] आकृति जोड़ें
    [chart] चार्ट जोड़ें
   *[text] टेक्स्ट जोड़ें
}
ui-undo-new-template = नया टेम्पलेट: { $name }
ui-undo-new-slide = नई स्लाइड: { $name }
ui-undo-grid = ग्रिड { $columns } × { $rows }
ui-undo-templates = टेम्पलेट: { $name }
ui-undo-move-template = टेम्पलेट खिसकाएँ
ui-undo-move-slide = स्लाइड खिसकाएँ
ui-undo-remove-slide = स्लाइड हटाएँ
ui-undo-remove = हटाएँ
ui-undo-cut = काटें
ui-undo-paste = चिपकाएँ
ui-undo-place-on-grid = ग्रिड पर रखें
ui-undo-move-on-grid = ग्रिड पर खिसकाएँ
ui-undo-resize-on-grid = ग्रिड पर आकार बदलें
ui-undo-change-anchor = एंकरिंग बदलें
ui-undo-move = खिसकाएँ
ui-undo-resize = आकार बदलें
ui-undo-edit-text = टेक्स्ट संपादित करें
ui-undo-dictation = डिक्टेशन
ui-undo-rename-set = सेट का नाम बदलें
ui-undo-rename-deck = प्रेज़ेंटेशन का नाम बदलें
ui-undo-theme = थीम बदलें
ui-undo-rename-template = टेम्पलेट का नाम बदलें
ui-undo-describe-template = टेम्पलेट का विवरण दें
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
ui-stage-edit-text = टेक्स्ट संपादित करें
ui-stage-hint-field = पुष्टि के लिए Enter · रद्द करने के लिए Esc
ui-stage-hint-block = पुष्टि के लिए ⌘ Enter · हर अनुच्छेद के लिए एक पंक्ति, बुलेट के लिए “- ”
ui-field = { $kind ->
    [title] शीर्षक
    [subtitle] उपशीर्षक
    [kicker] किकर
    [text] टेक्स्ट
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

## स्लाइड नोट्स

ui-notes = स्लाइड टूल
ui-stepper-less = { $label }: एक कम
ui-stepper-more = { $label }: एक और

## डिक्टेशन

ui-dictation = डिक्टेशन
ui-dictation-start = बोलकर लिखें
ui-dictation-start-title = बोलकर लिखें (⌘D)
ui-dictation-finish = डिक्टेशन समाप्त करें
ui-dictation-finish-title = समाप्त करें (⌘D)
ui-dictation-cancel = रद्द करें
ui-dictation-preparing = डिक्टेशन तैयार हो रहा है…
ui-dictation-heard-nothing = कुछ सुनाई नहीं दिया: माइक्रोफ़ोन के थोड़ा पास बोलें
ui-dictation-to-cursor = कर्सर पर
ui-dictation-to-field = { $field } बदलें
ui-dictation-to-block = चुने गए ब्लॉक के बाद
ui-dictation-new-block = नया टेक्स्ट ब्लॉक
ui-dictation-select-zone = एक क्षेत्र चुनें
ui-dictation-downloading = { $name } मॉडल डाउनलोड हो रहा है
ui-dictation-download-progress = { $percent }% · { $total } में से { $received } · सिर्फ़ एक बार
ui-dictation-listening = सुन रहा है · { $time }
ui-dictation-transcribing = आख़िरी वाक्य लिखा जा रहा है…
ui-dictation-stop-in = { $duration } में रुकेगा
ui-dictation-speak = बोलिए, टेक्स्ट यहाँ दिखेगा…
ui-dictation-auto-stop = { $duration } की चुप्पी के बाद रुकता है
ui-dictation-to-download = { $size } डाउनलोड करना है
ui-dictation-settings = डिक्टेशन सेटिंग्स
ui-dictation-model-title = पहचान मॉडल
ui-dictation-model-intro = Whisper, इसी कंप्यूटर पर (CTranslate2)। हर मॉडल कई भाषाएँ समझता है।
ui-dictation-note-turbo = अनुशंसित, तेज़ और सटीक
ui-dictation-note-large = सबसे सटीक, धीमा
ui-dictation-note-base = हल्का, छोटी मशीनों के लिए
ui-dictation-installed = इंस्टॉल है
ui-dictation-not-installed = डाउनलोड करना है
ui-dictation-language = बोली जाने वाली भाषा
ui-dictation-language-auto = स्वचालित पहचान
ui-dictation-auto-stop-label = इतनी देर की चुप्पी के बाद अपने आप रुकें
ui-dictation-silence = चुप्पी की अवधि
ui-dictation-phrases-note = आपके बोलते समय केवल बोले गए अंश ही, वाक्य-दर-वाक्य, लिखे जाते हैं।
ui-dictation-download-now = अभी डाउनलोड करें ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } सेकंड
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## टेम्पलेट चयनकर्ता

ui-picker-new-slide = नई स्लाइड
ui-picker-new-template = नया टेम्पलेट, इस पर आधारित…
ui-picker-set = टेम्पलेट
ui-picker-set-mine = { $name } · मेरे
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
ui-chart-add-row = + पंक्ति
ui-chart-add-column = + कॉलम
ui-chart-series = सीरीज़ { $n }

## फ़ाइलें

ui-file-open-prompt = खोलने के लिए फ़ाइल का पाथ (.oku-slides.pdf या .json)
ui-file-save-prompt = इस रूप में सहेजें (पूरा पाथ)

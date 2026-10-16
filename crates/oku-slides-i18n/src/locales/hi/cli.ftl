cli-reading = { $path } पढ़ा जा रहा है
cli-invalid-deck = अमान्य प्रेज़ेंटेशन: { $path }
cli-deck-valid = मान्य प्रेज़ेंटेशन: { $slides } स्लाइड
cli-deck-issue = समस्या: { $issue }
cli-rendered = { $pages } पेज → { $out } (कंपाइलेशन { $compile }, SVG { $svg })
cli-asset-added = एसेट { $path } → { $id }
cli-saved = { $path }: { $pages } पेज, PDF { $pdf } KB, स्रोत { $sources } KB
cli-opened = { $path }: { $slides } स्लाइड, { $assets } एसेट, { $fonts } फ़ॉन्ट
cli-extracted = { $dir } में निकाला गया
cli-stripped = { $path }: स्रोतों के बिना कॉपी
cli-pdf-modified = Oku Slides के बाद PDF को किसी अन्य टूल ने बदला है
cli-iterations = --iterations 0 से बड़ा होना चाहिए
cli-no-block = प्रेज़ेंटेशन में संपादित करने के लिए कोई ब्लॉक नहीं है
cli-blob-missing = स्रोत ब्लॉब नहीं मिला
cli-roundtrip-mismatch = दोबारा पढ़ा गया प्रेज़ेंटेशन सहेजे गए से अलग है
cli-bench-world = वातावरण बनाना (फ़ॉन्ट)          { $time }
cli-bench-first = पहला कंपाइलेशन                  { $time }  ({ $pages } पेज)
cli-bench-same = बिना बदलाव पुनः कंपाइलेशन       { $time }
cli-bench-edited-text = संपादित टेक्स्ट { $n }
cli-bench-edit = set_text के बाद पुनः कंपाइलेशन  { $time }
cli-bench-svg = हर स्लाइड का SVG                { $time }  ({ $each } प्रति स्लाइड)
cli-bench-pdf = PDF एक्सपोर्ट                    { $time }  ({ $size } KB)
cli-bench-embed = स्रोत ब्लॉब एम्बेड करना          { $time }
cli-bench-extract = ब्लॉब वापस पढ़ना                { $time }
cli-bench-roundtrip = प्रेज़ेंटेशन राउंड ट्रिप         समान
cli-bench-history = इतिहास का कदम                 औसतन { $mean } ms, p50 { $p50 } ms
cli-serving = Oku Slides (dev) http://{ $host }:{ $port } पर
cli-serving-token = { $url } खोलें (टोकन सर्वर रुकने तक मान्य है)
cli-models-dir = डिक्टेशन मॉडल: { $dir }
cli-stopped = सर्वर बंद हुआ
cli-not-found = नहीं मिला
cli-bad-request = अमान्य अनुरोध: { $reason }
cli-sse-full = पहले से ही बहुत सारे इवेंट स्ट्रीम खुले हैं
cli-forbidden = अनुरोध अस्वीकृत: यह स्थानीय इंटरफ़ेस से नहीं आया
cli-unauthorized = अनुरोध अस्वीकृत: एक्सेस टोकन नहीं है या गलत है
cli-exposed = { $host } पर सुन रहा है: सर्वर नेटवर्क से पहुँच योग्य है; केवल वे क्लाइंट सेवा पाते हैं जो इस बार चलाने का टोकन देते हैं। नेटवर्क की ज़रूरत न हो तो [serve] host को 127.0.0.1 पर रखें
cli-imported = { $path }: { $slides ->
       [one] { $slides } स्लाइड
      *[other] { $slides } स्लाइड
    }, { $pages ->
       [one] { $pages } पेज
      *[other] { $pages } पेज
    }, { $losses ->
       [one] { $losses } हानि
      *[other] { $losses } हानियाँ
    }, { $requests ->
       [one] { $requests } विजेट अनुरोध
      *[other] { $requests } विजेट अनुरोध
    }
cli-import-label = PowerPoint आयात
cli-import-pack-required = retrofit मोड में --pack ज़रूरी है: टेम्पलेट पैक की आईडी, फ़ोल्डर या नाम
cli-import-unknown-pack = अज्ञात टेम्पलेट पैक: { $pack }
cli-import-bad-pack = अपठनीय टेम्पलेट पैक { $pack }: इसकी pack.json टेम्पलेट पैक का वर्णन नहीं करती
cli-import-bad-theme = लक्ष्य थीम अपठनीय: इसके डिज़ाइन टोकन पढ़े नहीं जा सके
cli-import-write-failed = आयात किया गया डेक { $path } लिखा नहीं जा सका
cli-import-hifi-no-target = --pack और --theme केवल retrofit मोड के लिए हैं: उच्च-निष्ठा आयात (--mode hifi) अपना पैक और थीम ख़ुद बनाता है
cli-import-probe-mismatch = बनाया गया पैक घोषणा के अनुसार जाँच में नहीं मिला (विजेट { $widget }): यह आयातक की त्रुटि है
cli-import-probe-limit = बनाया गया पैक जाँच की सीमाएँ पार करता है (विजेट { $widget }): कॉन्फ़िगरेशन के [render] खंड में max_widgets, max_widget_kib या probe_budget_ms बढ़ाएँ
cli-import-probe-failed = रेंडर प्रक्रिया बनाए गए पैक की जाँच नहीं कर सकी: { $error }
cli-import-pack-missing = आयात ने डेक के लिए कोई पैक नहीं बनाया: यह आयात की त्रुटि है
cli-import-theme-missing = आयात ने डेक के लिए कोई थीम नहीं बनाई: यह आयात की त्रुटि है
cli-import-reference-hifi-only = --reference hifi मोड का हिस्सा है: retrofit आयात की तुलना संदर्भ रेंडर से नहीं होती
cli-import-bad-threshold = [import] अनुभाग का score_threshold शून्य या उससे अधिक की संख्या होना चाहिए, { $value } नहीं
cli-import-scored = { $witness } से दृश्य अंतर: औसत { $mean }, { $threshold } से ऊपर { $above ->
    [0] कोई स्लाइड नहीं
    [one] { $above } स्लाइड
   *[other] { $above } स्लाइड
}
cli-import-unscored = { $reason }

cli-about = Oku Slides प्रेज़ेंटेशन रेंडर और संग्रहीत करें
cli-arg-fonts = अतिरिक्त फ़ॉन्ट फ़ोल्डर (TTF/OTF); डिफ़ॉल्ट: सेटिंग्स में paths.fonts
cli-cmd-check = प्रेज़ेंटेशन जाँचें
cli-arg-deck = प्रेज़ेंटेशन (JSON)
cli-cmd-render = हर स्लाइड को एलिमेंट बॉक्स के साथ SVG में रेंडर करें
cli-arg-input = deck.json या .oku-slides.pdf फ़ाइल
cli-arg-out = आउटपुट फ़ोल्डर
cli-arg-theme = प्रेज़ेंटेशन की थीम बदलें: बिल्ट-इन या लाइब्रेरी की थीम, नाम से
cli-cmd-save = प्रेज़ेंटेशन को .oku-slides.pdf (PDF + स्रोत ब्लॉब) के रूप में सहेजें
cli-arg-output = लिखी जाने वाली फ़ाइल
cli-arg-assets = एसेट फ़ोल्डर: हर फ़ाइल एक sha256:… एसेट बनती है
cli-arg-embed-fonts = --fonts के फ़ॉन्ट भी स्रोतों में एम्बेड करें
cli-cmd-open = .oku-slides.pdf फिर से खोलें: प्रेज़ेंटेशन और उसके एसेट निकालें
cli-arg-extract = वह फ़ोल्डर जिसमें deck.json, assets/ और fonts/ लिखे जाएँ
cli-cmd-strip = .oku-slides.pdf को उसके स्रोत ब्लॉब के बिना कॉपी करें
cli-cmd-bench = कंपाइलेशन और एक्सपोर्ट समय मापें
cli-arg-iterations = प्रति चरण माप
cli-cmd-schema = ऑपरेशनों का JSON स्कीमा प्रिंट करें
cli-cmd-serve = डेवलपमेंट सर्वर: ब्राउज़र में इंटरफ़ेस, Tauri के बिना
cli-arg-serve-input = खोलने के लिए प्रेज़ेंटेशन (.oku-slides.pdf या .json)
cli-arg-port = पोर्ट; डिफ़ॉल्ट: सेटिंग्स में serve.port
cli-arg-ui = बने हुए इंटरफ़ेस का फ़ोल्डर (ui/dist)
cli-cmd-import = PowerPoint प्रेज़ेंटेशन (.pptx) को .oku-slides.pdf के रूप में आयात करें
cli-arg-pptx = PowerPoint प्रेज़ेंटेशन (.pptx)
cli-arg-mode = retrofit: मौजूदा टेम्पलेट पैक और थीम पर सामग्री; hifi: हूबहू प्रति, उसके लिए बनाए गए पैक और थीम पर
cli-arg-pack = लक्ष्य टेम्पलेट पैक: आईडी, पैक फ़ोल्डर (pack.json, widgets/) या नाम
cli-arg-import-theme = लक्ष्य थीम: बिल्ट-इन या लाइब्रेरी की थीम, आईडी या नाम से; डिफ़ॉल्ट: डिफ़ॉल्ट थीम
cli-arg-report = आयात रिपोर्ट (JSON) भी लिखें: हानियाँ, विकल्प, कवरेज
cli-arg-no-system-fonts = hifi मोड में, प्रस्तुति के फ़ॉन्ट सिस्टम फ़ॉन्ट में न खोजें
cli-arg-reference = hifi मोड में, हर स्लाइड की तुलना प्रस्तुति के संदर्भ रेंडर से करें: auto (डिफ़ॉल्ट, office2pdf), office2pdf, soffice (LibreOffice) या नहीं (none)

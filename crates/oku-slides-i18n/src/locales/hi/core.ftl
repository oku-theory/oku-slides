op-slide-not-found = स्लाइड नहीं मिली: { $subject }
op-element-not-found = एलिमेंट नहीं मिला: { $subject }
op-duplicate-id = पहचानकर्ता पहले से उपयोग में है: { $subject }
op-index-out-of-range = स्थिति सीमा से बाहर है
op-invalid-value = { $subject } के लिए अमान्य मान
op-invalid-value-because = { $subject } के लिए अमान्य मान: { $reason }
op-value-no-value = पैरामीटर का कोई मान नहीं है
op-value-no-dataset = चार्ट में कोई डेटासेट नहीं है
op-value-grid-tracks = ग्रिड में 1 से 24 कॉलम और पंक्तियाँ होती हैं
op-value-bad-item-id = आइटम की पहचान “/” के बिना एक गैर-खाली टेक्स्ट होती है
op-value-missing-item-id = हर आइटम की एक पहचान होनी चाहिए
op-value-too-deep = तत्व बहुत गहराई तक नेस्ट हैं
op-value-empty-path = पथ खाली है
op-value-not-text = टेक्स्ट या null अपेक्षित है
op-value-empty-frame = चौड़ाई और ऊँचाई 0 से अधिक होनी चाहिए
op-value-blocks-off-grid = फ़्लो स्लॉट स्लाइड व्यवस्थित करती है और ग्रिड उन्हें छिपा देता है; पहले हर तत्व को किसी खाली स्लॉट में ले जाएँ
op-value-outside-grid = सेल ग्रिड से बाहर है
op-value-not-tokens = ऑब्जेक्ट या null अपेक्षित है
op-value-not-a-slot = यह पैरामीटर स्लॉट नहीं है
op-value-out-of-bounds = संख्या अनुमत सीमा से बाहर है
op-value-unreadable = मान अपेक्षित रूप में नहीं है
op-value-valign = top, middle या bottom अपेक्षित है
op-value-path = पथ M, L, C, Q और Z कमांड और उनकी संख्याओं से बनता है, और M से शुरू होता है
op-unknown-widget = अज्ञात विजेट: { $subject }
op-unknown-namespace = इस डेक में { $pack } पैक नहीं है।
op-pack-required = { $pack } पैक की ज़रूरत { $by } को है; पहले उसे हटाएँ।
op-pack-in-use = { $pack } पैक अभी भी { $count } आइटम में इस्तेमाल हो रहा है।
op-pack-requirement-missing = { $pack } पैक को { $required } चाहिए, जो इस डेक में नहीं है।
op-base-required = हर डेक में बुनियादी विजेट पैक बना रहता है।
op-pack-kind-conflict = इस डेक में पहले से दूसरे प्रकार (विजेट या लेआउट) का { $pack } पैक है।
op-theme-cycle = थीम एक लूप में एक-दूसरे का विस्तार करती हैं: { $chain }।
op-theme-closure = इस डेक में पिन की गई थीम ठीक-ठीक इसकी थीम और उसके पैरेंट नहीं हैं ({ $namespace })।
op-unknown-param = अज्ञात पैरामीटर: { $subject }
op-param-type-because = { $subject } के लिए अमान्य मान: { $reason }
op-param-shape-text = { $nullable ->
    [yes] एक टेक्स्ट या null अपेक्षित है
   *[no] एक टेक्स्ट अपेक्षित है
}
op-param-shape-choice = { $nullable ->
    [yes] इसके मानों में से एक या null अपेक्षित है
   *[no] इसके मानों में से एक अपेक्षित है
}
op-param-shape-paragraphs = { $nullable ->
    [yes] अनुच्छेदों की सूची या null अपेक्षित है
   *[no] अनुच्छेदों की सूची अपेक्षित है
}
op-param-shape-elements = { $nullable ->
    [yes] तत्वों की सूची या null अपेक्षित है
   *[no] तत्वों की सूची अपेक्षित है
}
op-param-shape-flag = { $nullable ->
    [yes] एक बूलियन या null अपेक्षित है
   *[no] एक बूलियन अपेक्षित है
}
op-param-shape-number = { $nullable ->
    [yes] एक संख्या या null अपेक्षित है
   *[no] एक संख्या अपेक्षित है
}
op-param-shape-bounded-number = { $nullable ->
    [yes] अपनी सीमाओं के भीतर एक संख्या या null अपेक्षित है
   *[no] अपनी सीमाओं के भीतर एक संख्या अपेक्षित है
}
op-param-shape-grid = { $nullable ->
    [yes] एक ग्रिड या null अपेक्षित है
   *[no] एक ग्रिड अपेक्षित है
}
op-param-shape-chart-option = { $nullable ->
    [yes] एक चार्ट विकल्प या null अपेक्षित है
   *[no] एक चार्ट विकल्प अपेक्षित है
}
op-param-shape-table-option = { $nullable ->
    [yes] एक तालिका विकल्प या null अपेक्षित है
   *[no] एक तालिका विकल्प अपेक्षित है
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] एक चार्ट या तालिका विकल्प या null अपेक्षित है
   *[no] एक चार्ट या तालिका विकल्प अपेक्षित है
}
op-param-shape-items = { $nullable ->
    [yes] आइटमों की सूची या null अपेक्षित है
   *[no] आइटमों की सूची अपेक्षित है
}
op-param-shape-list = { $nullable ->
    [yes] एक सूची या null अपेक्षित है
   *[no] एक सूची अपेक्षित है
}
op-param-shape-add-element-only = { $nullable ->
    [yes] तत्व add_element से जोड़े जाते हैं, या null
   *[no] तत्व add_element से जोड़े जाते हैं
}
op-param-shape-set-param-only = { $nullable ->
    [yes] मान set_param से सेट किया जाता है, या null
   *[no] मान set_param से सेट किया जाता है
}
op-param-shape-path-option = { $nullable ->
    [yes] आकृति का पथ (d कुंजी वाला ऑब्जेक्ट) या null अपेक्षित है
   *[no] आकृति का पथ (d कुंजी वाला ऑब्जेक्ट) अपेक्षित है
}
op-item-not-found = { $subject } नाम का कोई आइटम नहीं है
op-list-full = सूची { $subject } भरी हुई है (अधिकतम { $max } आइटम)
op-list-minimum = सूची { $subject } में कम से कम { $min } आइटम होने चाहिए
op-list-cell-too-long = सूची { $subject } की एक कोशिका बहुत लंबी है (कॉलम { $column })
op-list-id-too-long = सूची { $subject } के एक आइटम की पहचान बहुत लंबी है
op-list-too-many-items = सूची { $subject } में बहुत अधिक आइटम हैं (अधिकतम { $max })
op-slot-rejects = { $subject } यहाँ स्वीकार्य नहीं है
op-wrong-category = { $subject } का यहाँ उपयोग नहीं किया जा सकता
op-choice-out-of-range = { $subject } के लिए यह मान अनुमत नहीं है
transaction-refused = ऑपरेशन { $index } अस्वीकृत: { $reason }

sources-unreadable = स्रोत पढ़े नहीं जा सकते
sources-not-sources = यह Oku Slides स्रोत आर्काइव नहीं है
sources-unsupported-version = ये स्रोत Oku Slides के नए संस्करण से लिखे गए थे
sources-bad-name = स्रोतों में अस्वीकृत फ़ाइल नाम
sources-missing-deck = स्रोतों में प्रेज़ेंटेशन नहीं है
sources-too-large = इस फ़ाइल के स्रोत खोलने के लिए बहुत बड़े हैं
sources-too-deep = यह डेक तत्वों को बहुत गहराई तक नेस्ट करता है

issue-empty-slide-id = खाली स्लाइड पहचानकर्ता
issue-duplicate-slide-id = दोहराया गया स्लाइड पहचानकर्ता
issue-duplicate-element-id = दोहराया गया एलिमेंट पहचानकर्ता
issue-empty-element-id = खाली पहचानकर्ता
issue-cell-outside-grid = सेल ग्रिड से बाहर है
issue-empty-frame = शून्य आकार का फ़्रेम
issue-free-without-frame = फ़्रेम के बिना मुक्त एलिमेंट
issue-unknown-widget = अज्ञात विजेट: { $kind }
issue-wrong-category = { $kind } का यहाँ उपयोग नहीं हो सकता
issue-unknown-param = अज्ञात पैरामीटर: { $param }
issue-param-type = { $param } के लिए अमान्य मान
issue-choice-out-of-range = { $param } के लिए अनुमत मान नहीं
issue-slot-rejects = { $kind } यहाँ स्वीकार नहीं है
issue-placed-in-flow = व्यवस्थित सूची में फ़्रेम के साथ रखा गया तत्व
issue-unknown-bind = { $param } एक अज्ञात डेक सेटिंग से जुड़ा है
issue-slot-too-deep = तत्व बहुत गहराई तक नेस्टेड हैं
issue-invalid-style = अमान्य शैली मान: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = एसेट गायब है: { $asset }
issue-pack-version-mismatch = बदला हुआ पैक संस्करण हटाया गया: { $version }
issue-pack-version-missing = पैक संस्करण स्रोतों में नहीं है: { $version }
issue-pack-pin-mismatch = एम्बेड किया गया पैक { $pack } उससे मेल नहीं खाता जिसकी डेक को अपेक्षा है।
issue-pack-tokens-invalid = एम्बेड किए गए पैक { $pack } में एक अमान्य कंपोनेंट टोकन है: { $reason }
issue-pack-kind-mismatch = एम्बेड किया गया पैक { $pack } उस प्रकार (विजेट या लेआउट) का नहीं है जिसकी डेक को अपेक्षा है।
issue-pack-kind-conflict = नेमस्पेस { $pack } विजेट पैक और लेआउट पैक दोनों के रूप में पिन है: केवल एक ही इस्तेमाल हो सकता है।
issue-pack-requirement-missing = { $pack } पैक को { $required } चाहिए, जो इस डेक में नहीं है।
issue-theme-version-mismatch = बदला हुआ थीम संस्करण हटाया गया: { $version }
issue-theme-version-missing = थीम संस्करण स्रोतों में नहीं है: { $version }
issue-theme-pin-mismatch = एम्बेड की गई थीम { $namespace } उससे मेल नहीं खाती जिसकी डेक को अपेक्षा है।
issue-theme-closure = इस डेक में पिन की गई थीम ठीक-ठीक इसकी थीम और उसके पैरेंट नहीं हैं ({ $namespace })।
pack-token-entry-invalid = पैक मैनिफ़ेस्ट में अमान्य कंपोनेंट टोकन “{ $path }”।
pack-manifest-json = पैक मैनिफ़ेस्ट पढ़ा नहीं जा सका: { $reason }
issue-unsupported-link = असमर्थित लिंक: { $link }
issue-highlight-zero = हाइलाइट रंगों की गिनती 1 से होती है
issue-unreadable-element = अपठनीय तत्व, अलग रखा गया
issue-slot-repaired = ज्ञात स्लॉट के बिना तत्व, { $slot } में रखा गया
issue-unreadable-item = अपठनीय सूची आइटम, अलग रखा गया
issue-unreadable-row = अपठनीय तालिका पंक्ति, अलग रखी गई
issue-data-truncated = डेटा बहुत बड़ा है, पढ़ते समय काटा गया
issue-unreadable-slide = अपठनीय स्लाइड, अलग रखी गई
issue-unreadable-field = प्रस्तुति का अपठनीय फ़ील्ड { $field }, रीसेट किया गया
op-invalid-chart = चार्ट { $subject }: { $reason }
op-invalid-table = तालिका { $subject }: { $reason }

replica-corrupted = इस फ़ाइल में प्रस्तुति का इतिहास क्षतिग्रस्त है
replica-too-large = इस फ़ाइल में प्रस्तुति का इतिहास खोलने के लिए बहुत बड़ा है
replica-write = प्रस्तुति का इतिहास अपडेट नहीं हो सका
replica-unrelated = इन दोनों प्रस्तुतियों का कोई साझा मूल नहीं है
replica-unsupported-version = यह प्रस्तुति ऐसे प्रारूप का उपयोग करती है जिसे Oku Slides का यह संस्करण नहीं पढ़ सकता
replica-too-deep = यह प्रस्तुति सहेजी नहीं जा सकती: इसके इतिहास में हटाए गए तत्व बहुत गहराई तक नेस्ट हैं। अपने पिछले स्थानांतरण पूर्ववत करें, फिर दोबारा सहेजें।

widget-unreadable = विजेट घोषणाएँ पढ़ी नहीं जा सकीं
widget-fault-empty-name = विजेट { $widget }: किसी विजेट या पैरामीटर का कोई नाम नहीं है
widget-fault-duplicate-widget = विजेट { $widget } दो बार घोषित किया गया है
widget-fault-duplicate-param = विजेट { $widget } एक ही पैरामीटर को दो बार घोषित करता है
widget-fault-empty-choice = विजेट { $widget }: किसी चॉइस का कोई मान नहीं है
widget-fault-default-not-in-choice = विजेट { $widget }: चॉइस का डिफ़ॉल्ट मान उसके मानों में शामिल नहीं है
widget-fault-default-wrong-type = विजेट { $widget }: डिफ़ॉल्ट मान उसके पैरामीटर के प्रकार से मेल नहीं खाता
widget-fault-bind-not-plain = विजेट { $widget }: केवल सादा टेक्स्ट पैरामीटर को प्रस्तुति से जोड़ा जा सकता है
widget-fault-unknown-slot-kind = विजेट { $widget }: एक स्लॉट अज्ञात एलिमेंट विजेट स्वीकार करता है
widget-fault-empty-list = विजेट { $widget }: सूची में कम से कम एक कॉलम होना चाहिए
widget-fault-column-type = विजेट { $widget }: सूची का कॉलम plain, rich, number, flag, choice या asset (या एक नेस्टेड p.list, एक स्तर) हो सकता है; प्रति आइटम मुक्त सामग्री के लिए चाइल्ड विजेट के साथ p.slot का उपयोग करें
widget-fault-reserved-name = विजेट { $widget }: आरक्षित नाम: id, kind, या / वाला नाम
widget-fault-list-bounds = विजेट { $widget }: सूची का अधिकतम उसके न्यूनतम से कम है
widget-fault-list-default-too-large = विजेट { $widget }: किसी सूची का डिफ़ॉल्ट सूचियों की आकार सीमाओं से अधिक है
widget-fault-list-depth = विजेट { $widget }: नेस्टेड सूची में सूची कॉलम नहीं हो सकता; सूचियाँ केवल एक स्तर तक नेस्ट होती हैं
widget-fault-list-default-below-min = विजेट { $widget }: किसी नेस्टेड सूची के डिफ़ॉल्ट में उसकी न्यूनतम संख्या से कम आइटम हैं
widget-fault-slide-sizing = विजेट { $widget } एक स्लाइड है: grow या typography केवल तत्व विजेट घोषित करते हैं
widget-fault-pack-category = विजेट { $widget } अपने पैक से मेल नहीं खाता: लेआउट पैक में स्लाइड विजेट होते हैं, विजेट पैक में तत्व विजेट होते हैं
widget-fault-unqualified = विजेट { $widget } का कोई पैक नेमस्पेस नहीं है
issue-empty-item-id = सूची { $param } में एक आइटम की पहचान नहीं है
issue-duplicate-item-id = सूची { $param } में आइटम की पहचान दोहराई गई है
issue-invalid-item-id = सूची { $param } में स्लैश वाला आइटम पहचानकर्ता है
issue-list-bounds = सूची { $param } में आइटम बहुत कम या बहुत ज़्यादा हैं

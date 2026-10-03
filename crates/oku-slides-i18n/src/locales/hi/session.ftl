session-read = { $path } पढ़ा नहीं जा सकता
session-write = { $path } लिखा नहीं जा सकता
session-not-oku-slides = यह PDF Oku Slides से नहीं बनाया गया था: इसे संपादित नहीं किया जा सकता
session-invalid-deck = प्रेज़ेंटेशन का JSON अमान्य है
session-not-rendered = प्रेज़ेंटेशन अभी रेंडर नहीं हुआ है
session-pack-disabled = पैक “{ $pack }” के विजेट इस सत्र के लिए बंद कर दिए गए: { $reason }। प्रेज़ेंटेशन मूल विजेट के साथ दिखाया गया है।
session-pack-missing = इस प्रेज़ेंटेशन का टेम्पलेट पैक फ़ाइल में नहीं है: इसे मूल विजेट के साथ दिखाया गया है।
session-no-path = अभी कोई फ़ाइल नहीं: सहेजने की जगह चुनें
session-no-slide = स्लाइड नहीं मिली: { $id }
session-no-chart = यहाँ कोई चार्ट नहीं है: { $subject }
session-builtin-template-set = बिल्ट-इन टेम्पलेट सेट हटाया नहीं जा सकता
session-no-templates-dir = आपके टेम्पलेट के लिए कोई फ़ोल्डर नहीं
session-unknown-pack = टेम्पलेट पैक नहीं मिला: { $id }
session-pack-version-missing = टेम्पलेट पैक { $id } का संस्करण { $version } नहीं मिला
session-invalid-pack = टेम्पलेट पैक पढ़ा नहीं जा सकता: { $id }
session-pack-id-mismatch = यह pack.json टेम्पलेट पैक { $found } का है, { $id } का नहीं
session-editing-templates = पहले टेम्पलेट का संपादन पूरा करें
session-not-editing-templates = कोई टेम्पलेट सेट संपादित नहीं हो रहा है
session-response = उत्तर लिखा नहीं जा सका
session-unknown-tab = यह टैब अब खुला नहीं है।
session-unknown-window = यह विंडो अब खुली नहीं है।
session-single-document = यहाँ केवल एक प्रस्तुति खुली रह सकती है।
session-last-window = आख़िरी विंडो खुली रहती है: इसके बजाय ऐप बंद करें।
session-unsaved = “{ $title }” में बिना सहेजे बदलाव हैं।
session-bad-request = अमान्य अनुरोध: { $reason }
session-history-not-open = इतिहास खुला नहीं है
session-in-history = प्रस्तुति बदलने से पहले इतिहास बंद करें
session-agent-form = एजेंट का अनुरोध अमान्य है: { $reason }

deck-untitled = बिना शीर्षक
deck-new-title = नया प्रेज़ेंटेशन
deck-new-slide-title = स्लाइड का शीर्षक
deck-new-slide-text = बताइए कि आप क्या दिखाना चाहते हैं।
templates-default-name = मेरे टेम्पलेट
templates-copy-name = { $name } (कॉपी)
templates-adopt-label = टेम्पलेट: { $name }
session-widgets-broken = ये विजेट कंपाइल नहीं होते, इसलिए छोड़ दिए गए: { $names }। पैक के बाकी विजेट बने रहते हैं।
session-invalid-widget-name = विजेट का नाम अमान्य है: “{ $name }”। छोटे अक्षर, अंक और हाइफ़न, या _commun इस्तेमाल करें।
session-widget-too-large = विजेट “{ $name }” { $max } KiB से बड़ा है।
session-widget-nul = विजेट “{ $name }” में NUL वर्ण है।
session-too-many-widgets = किसी पैक में { $max } से ज़्यादा विजेट फ़ाइलें नहीं हो सकतीं।
session-widget-in-use = विजेट “{ $name }” इन टेम्पलेट में इस्तेमाल होता है: { $templates }।
session-unknown-widget = विजेट नहीं मिला: “{ $name }”।
session-widget-exists = “{ $name }” नाम का विजेट पहले से मौजूद है।
widgets-edit-label = विजेट: { $name }
widget-preview-plain = नमूना पाठ
widget-preview-rich = विजेट को संदर्भ में देखने के लिए एक नमूना अनुच्छेद।
pack-notice-older = यह प्रेज़ेंटेशन “{ $name }” का आपके संस्करण से पुराना संस्करण उपयोग करता है।
pack-notice-newer = यह प्रेज़ेंटेशन “{ $name }” का आपके संस्करण से नया संस्करण उपयोग करता है।
pack-notice-parallel = यह प्रेज़ेंटेशन “{ $name }” का ऐसा संस्करण उपयोग करता है जो आपके संस्करण के समानांतर बना है।
pack-notice-use-mine = मेरे संस्करण पर जाएँ
pack-notice-make-current = उनके संस्करण को मेरा वर्तमान संस्करण बनाएँ
template-default-name = टेम्पलेट
history-restore-label = { $date } का संस्करण पुनर्स्थापित करें
notice-merged = { $file } के साथ मिलाया गया — आयात किए गए बदलाव: { $count } ({ $authors })।
notice-merged-anonymous = { $file } के साथ मिलाया गया — आयात किए गए बदलाव: { $count }।
notice-same-deck-open = यह प्रस्तुति पहले से टैब “{ $title }” में खुली है। दोनों को मिलाने के लिए वहाँ “PDF के साथ मिलाएँ…” का उपयोग करें।
notice-modified-elsewhere = Oku Slides के बाद इस PDF को किसी अन्य टूल ने बदला है: हो सकता है इसके स्रोत PDF रीडर में दिखने वाली सामग्री से मेल न खाएँ।
notice-open-failed = { $path } नहीं खुल सका: { $reason }
notice-tab-resume-failed = इस टैब की स्थिति बहाल नहीं की जा सकी: उसे अलग रख दिया गया है, और टैब एक खाली प्रस्तुति से फिर शुरू होता है।
notice-unreadable-parts = इस प्रस्तुति के कुछ हिस्से पढ़े नहीं जा सके और अलग रखे गए ({ $count })।

log-document-opened = दस्तावेज़ खोला गया
log-document-saved = दस्तावेज़ सहेजा गया

## Panneau de droite (surface A2UI)

session-panel-template = यह टेम्पलेट
session-panel-name = नाम
session-panel-description = विवरण
session-panel-grid = ग्रिड
session-panel-columns = कॉलम
session-panel-rows = पंक्तियाँ

## Menu

menu-edit = संपादन
menu-window = विंडो
menu-quit = छोड़ें

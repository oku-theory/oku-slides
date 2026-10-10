pdf-unreadable = PDF पढ़ा नहीं जा सकता
pdf-unsupported-version = यह PDF Oku Slides के नए संस्करण से लिखा गया था
pdf-corrupted = इस PDF के स्रोत क्षतिग्रस्त हैं
pdf-write = PDF लिखा नहीं जा सका

render-compile = प्रेज़ेंटेशन रेंडर नहीं हो सका
render-pdf = PDF एक्सपोर्ट विफल रहा
render-no-page = यह पेज मौजूद नहीं है
render-image = स्लाइड की छवि का उत्पादन नहीं किया जा सका

render-setup = रेंडरर शुरू नहीं हो सका
render-missing-image = इमेज गायब है
diagnostic-error = त्रुटि
diagnostic-warning = चेतावनी
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = संकेत: { $hint }
render-chart-failed = चार्ट बनाया नहीं जा सका

registry-bad-pack-id = अमान्य टेम्पलेट पैक पहचानकर्ता: { $id }
registry-compile = टेम्पलेट पैक के विजेट कंपाइल नहीं हो सके
registry-missing = टेम्पलेट पैक अपने विजेट घोषित नहीं करता
render-timeout = रेंडरिंग में { $seconds } सेकंड से अधिक लगे और उसे रोक दिया गया
render-out-of-memory = रेंडरिंग को { $mib } MiB से अधिक मेमोरी चाहिए थी और उसे रोक दिया गया
render-worker-lost = रेंडरिंग प्रोसेस रुक गया
render-frame = रेंडरिंग प्रोसेस का संदेश पढ़ा नहीं जा सका
render-page-too-large = यह स्लाइड प्रीव्यू के लिए बहुत बड़ी है ({ $mib } MiB से अधिक, अक्सर किसी बहुत बड़ी छवि के कारण)। इसे अब भी PDF या PNG के रूप में एक्सपोर्ट किया जा सकता है।
render-stale = रेंडरिंग बदल दी गई; फिर से कोशिश करें
render-pack-disabled = पैक “{ $pack }” के विजेट इस सत्र के लिए बंद हैं
registry-schema-remote = विजेट रजिस्ट्री अमान्य है: { $message }
probe-not-probed = अभी जाँचा नहीं गया: पैक के विजेट के लिए तय समय समाप्त हो गया। अगले रेंडर पर इसकी जाँच होगी।
probe-too-many-files = पैक में { $max } से अधिक विजेट फ़ाइलें हैं: यह फ़ाइल लोड नहीं की गई।
probe-too-large = { $max } KiB से बड़ा: लोड नहीं किया गया।

import-unreadable = यह प्रस्तुति पढ़ी नहीं जा सकी
import-not-a-presentation = यह फ़ाइल PowerPoint प्रस्तुति नहीं है
import-protected = यह प्रस्तुति एन्क्रिप्टेड है या पुराने .ppt फ़ॉर्मैट में है: पहले इसे .pptx के रूप में सहेजें
import-too-large = यह प्रस्तुति आयात करने के लिए बहुत बड़ी है
import-loss-list-levels = सूची के स्तर एक स्तर में बदल दिए गए
import-loss-merged-cells = मर्ज किए गए सेल अलग कर दिए गए
import-loss-percent-stacked = 100% स्टैक्ड चार्ट स्टैक्ड के रूप में दिखाया गया
import-loss-run-style = { $effect ->
    [underline] रेखांकन खो गया
    [strike] काट-रेखा खो गई
    [baseline] सुपरस्क्रिप्ट या सबस्क्रिप्ट खो गया
    [caps] बड़े अक्षर खो गए
   *[highlight] हाइलाइट खो गया
}
import-loss-transform = घुमाव, पलटाव या क्रॉप खो गया
import-loss-color-dropped = रंग { $color } के लिए थीम में कोई निकट रंग नहीं है: थीम का रंग इस्तेमाल हुआ
import-loss-text-truncated = सीमाओं में रखने के लिए टेक्स्ट छोटा किया गया
import-loss-table-truncated = सीमाओं में रखने के लिए तालिका छोटी की गई
import-loss-opaque = { $reason ->
    [media] वीडियो या ध्वनि आयात नहीं हुई (कोई वैकल्पिक छवि नहीं)
    [ole] एम्बेड किया गया ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [math] समीकरण आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [smartart] SmartArt आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
   *[unknown] असमर्थित ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
}
import-loss-unsupported-image = { $format } फ़ॉर्मैट की छवि आयात नहीं हुई
import-loss-hidden-slide = छिपी स्लाइड दिखाई देने वाली के रूप में आयात हुई
import-loss-effects = छाया, प्रतिबिंब और 3D प्रभाव खो गए
import-loss-series-dropped = पाई चार्ट केवल एक शृंखला दिखाता है: { $kept } रखी गई, { $dropped } हटाई गईं
import-loss-repeated-decor = { $slides ->
   [one] { $slides } स्लाइड पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
  *[other] { $slides } स्लाइडों पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
}
import-loss-no-slot = सामग्री छोड़ दी गई: चुने गए टेम्पलेट में इसके लिए जगह नहीं है
import-warning-effect-ignored = प्रभाव आयात नहीं हुआ: { $detail }
import-warning-approximated = अनुमानित रूप में दिखाया गया: { $detail }
import-warning-unreadable-item = अपठनीय आइटम छोड़ा गया: { $detail }
import-warning-unreadable-part = अपठनीय भाग: { $detail }
import-warning-missing-part = अनुपस्थित भाग: { $detail }
import-warning-unsupported-chart = असमर्थित चार्ट: { $detail }
import-warning-unsupported-image = असमर्थित छवि फ़ॉर्मैट: { $detail }
import-warning-unknown-table-style = अज्ञात तालिका शैली: { $detail }
import-warning-unsafe-link = लिंक अस्वीकार किया गया: { $detail }
import-warning-nesting-too-deep = नेस्टिंग बहुत गहरी, काटी गई: { $detail }
import-warning-font-embedded-unreadable = एम्बेड किया गया फ़ॉन्ट अपठनीय: { $detail }
import-warning-truncated = बहुत अधिक चेतावनियाँ: बाकी छोड़ दी गईं

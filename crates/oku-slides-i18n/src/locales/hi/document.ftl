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
registry-base-pack-broken = अंतर्निहित विजेट पैक क्षतिग्रस्त है ({ $file }); ऐप को फिर से इंस्टॉल करें।
registry-duplicate-pack = प्रेज़ेंटेशन के दो विजेट पैक { $pack } साझा करते हैं।
registry-compile = टेम्पलेट पैक के विजेट कंपाइल नहीं हो सके
registry-missing = टेम्पलेट पैक अपने विजेट घोषित नहीं करता
render-timeout = रेंडरिंग में { $seconds } सेकंड से अधिक लगे और उसे रोक दिया गया
render-out-of-memory = रेंडरिंग को { $mib } MiB से अधिक मेमोरी चाहिए थी और उसे रोक दिया गया
render-worker-lost = रेंडरिंग प्रोसेस रुक गया
render-frame = रेंडरिंग प्रोसेस का संदेश पढ़ा नहीं जा सका
render-page-too-large = यह स्लाइड प्रीव्यू के लिए बहुत बड़ी है ({ $mib } MiB से अधिक, अक्सर किसी बहुत बड़ी छवि के कारण)। इसे अब भी PDF या PNG के रूप में एक्सपोर्ट किया जा सकता है।
render-stale = रेंडरिंग बदल दी गई; फिर से कोशिश करें
render-pack-disabled = पैक “{ $pack }” के विजेट इस सत्र के लिए बंद हैं
render-protocol = रेंडरिंग प्रक्रिया Oku Slides के किसी दूसरे संस्करण की है; ऐप्लिकेशन फिर से शुरू करें
render-remote = रेंडरिंग प्रक्रिया अनुरोध पूरा नहीं कर सकी
registry-schema-remote = विजेट रजिस्ट्री अमान्य है: { $message }
probe-not-probed = अभी जाँचा नहीं गया: पैक के विजेट के लिए तय समय समाप्त हो गया। अगले रेंडर पर इसकी जाँच होगी।
probe-too-many-files = पैक में { $max } से अधिक विजेट फ़ाइलें हैं: यह फ़ाइल लोड नहीं की गई।
probe-too-large = { $max } KiB से बड़ा: लोड नहीं किया गया।

import-unreadable = यह प्रस्तुति पढ़ी नहीं जा सकी
import-not-a-presentation = यह फ़ाइल PowerPoint प्रस्तुति नहीं है
import-protected = यह प्रस्तुति एन्क्रिप्टेड है या पुराने .ppt फ़ॉर्मैट में है: पहले इसे .pptx के रूप में सहेजें
import-too-large = यह प्रस्तुति आयात करने के लिए बहुत बड़ी है
import-output-too-large = परिणामी प्रस्तुति आयात के लिए अनुमत आकार से बड़ी हो जाएगी
import-loss-list-levels = सूची के स्तर एक स्तर में बदल दिए गए
import-loss-merged-cells = मर्ज किए गए सेल अलग कर दिए गए
import-loss-percent-stacked = 100% स्टैक्ड चार्ट स्टैक्ड के रूप में दिखाया गया
import-loss-chart-format = चार्ट का संख्या फ़ॉर्मैट { $format } नहीं रखा गया
import-loss-run-style = { $effect ->
    [underline] रेखांकन खो गया
    [strike] काट-रेखा खो गई
    [baseline] सुपरस्क्रिप्ट या सबस्क्रिप्ट खो गया
    [caps] बड़े अक्षर खो गए
    [font] एक अंश का फ़ॉन्ट बॉक्स के फ़ॉन्ट से बदला गया
    [tracking] एक अंश का अक्षर-अंतराल बॉक्स के अंतराल से बदला गया
    [weight] सामान्य अंश बॉक्स की तरह बोल्ड दिखाया गया
    [slant] सीधा अंश बॉक्स की तरह इटैलिक दिखाया गया
   *[highlight] हाइलाइट खो गया
}
import-loss-transform = घुमाव, पलटाव या क्रॉप खो गया
import-loss-color-dropped = रंग { $color } के लिए थीम में कोई निकट रंग नहीं है: थीम का रंग इस्तेमाल हुआ
import-loss-text-truncated = सीमाओं में रखने के लिए टेक्स्ट छोटा किया गया
import-loss-table-truncated = सीमाओं में रखने के लिए तालिका छोटी की गई
import-loss-chart-truncated = सीमाओं में रखने के लिए चार्ट का डेटा छोटा किया गया
import-loss-opaque = { $reason ->
    [media] वीडियो या ध्वनि आयात नहीं हुई (कोई वैकल्पिक छवि नहीं)
    [ole] एम्बेड किया गया ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [math] समीकरण आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [smartart] SmartArt आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
   *[unknown] असमर्थित ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
}
import-loss-smartart-layout = ड्रॉइंग के बिना SmartArt: इसका टेक्स्ट कार्ड के रूप में रखा गया, आरेख का लेआउट खो गया
import-loss-unsupported-image = { $format } फ़ॉर्मैट की छवि आयात नहीं हुई
import-loss-hidden-slide = छिपी स्लाइड दिखाई देने वाली के रूप में आयात हुई
import-loss-effects = छाया, प्रतिबिंब और 3D प्रभाव खो गए
import-loss-series-dropped = पाई चार्ट केवल एक शृंखला दिखाता है: { $kept } रखी गई, { $dropped } हटाई गईं
import-loss-repeated-decor = { $slides ->
   [one] { $slides } स्लाइड पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
  *[other] { $slides } स्लाइडों पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
}
import-loss-no-slot = सामग्री छोड़ दी गई: चुने गए टेम्पलेट में इसके लिए जगह नहीं है
import-loss-moved-to-notes = पाठ वक्ता नोट्स में ले जाया गया: आवरण में इसके लिए जगह नहीं है
import-loss-items-dropped = स्लाइड पर बहुत अधिक तत्व: छोड़े गए तत्व: { $dropped }
import-loss-page-fitted = स्रोत पृष्ठ { $width } × { $height } pt, 16:9 नहीं है: सामग्री का आकार बदलकर बीच में रखा गया
import-loss-paragraph-style = अनुच्छेद का संरेखण, पंक्ति अंतर, अनुच्छेद अंतर या इंडेंट बॉक्स जैसा किया गया
import-loss-shape-style = { $effect ->
    [gradient] ग्रेडिएंट एक रंग में दिखाया गया
    [picture] भरण चित्र आकृति के फ़्रेम तक खींचा गया
    [dash] डैश रेखा सतत दिखाई गई
    [arrow] रेखा के तीर-शीर्ष नहीं बनाए गए
   *[geometry] अज्ञात आकृति आयत के रूप में दिखाई गई
}
import-loss-layout-dropped = एक पैक के लिए बहुत अधिक लेआउट: “{ $layout }” का कोई टेम्पलेट नहीं; इसकी स्लाइडें खाली विजेट लेती हैं और इसकी सजावट तत्वों के रूप में पाती हैं
import-loss-decor-truncated = पैक फ़ाइल के आकार में समाने के लिए सजावट काटी गई: छोड़े गए तत्व: { $omitted }
import-loss-table-style = तालिका की शैली अनुमानित: इसकी अपनी सेल पृष्ठभूमि, किनारे, स्तंभ चौड़ाई, पंक्ति ऊँचाई या पाठ आकार खो गए
import-loss-overflow = तत्व “{ $element }” स्लाइड के किनारे से बाहर जाता है
import-loss-empty-data = बिना डेटा की तालिका या चार्ट: कुछ नहीं रखा गया
import-reference-unavailable = कोई दृश्य स्कोर नहीं: { $cause ->
    [timeout] संदर्भ रेंडर में बहुत समय लगा
    [memory] संदर्भ रेंडर ने बहुत अधिक मेमोरी ली
    [pages] संदर्भ रेंडर में प्रस्तुति के { $deck } के बदले { $reference ->
        [one] { $reference } पेज है
       *[other] { $reference } पेज हैं
    }
    [missing] LibreOffice (soffice) नहीं मिला; import.soffice में उसका पथ दें
   *[failed] संदर्भ रेंडरर प्रस्तुति नहीं पढ़ सका
}
import-region-suspect = अंतर अलग रखा गया, संदर्भ रेंडरर की एक ज्ञात त्रुटि: { $limit ->
    [autofit] जब PowerPoint ने कोई स्केल नहीं सहेजा हो, तो यह बॉक्स में फ़िट किए गए टेक्स्ट को छोटा कर देता है
    [tracking] यह अक्षर-अंतराल वाले टेक्स्ट को अधिक चौड़ा बनाता है
    [chart] यह चार्ट केवल लगभग बनाता है
    [default_font] जिस टेक्स्ट का फ़ॉन्ट केवल प्रस्तुति के डिफ़ॉल्ट बताते हैं, उसे यह अपने वैकल्पिक फ़ॉन्ट में बनाता है
    [no_substitute] इस फ़ॉन्ट का कोई विकल्प इसके पास नहीं है, इसलिए यह इसे दूसरे फ़ॉन्ट में बनाता है
    [smart_art_style] यह SmartArt का टेक्स्ट अपनी ही शैली में बनाता है
   *[smart_art] यह SmartArt का टेक्स्ट बहुत छोटा बनाता है
}
import-font-substituted = { $family } को मिलते-जुलते माप वाले { $by } से बदला गया
import-font-missing = { $family } नहीं मिला: टेक्स्ट { $by } का उपयोग करता है
import-font-restricted = { $family }: इसका लाइसेंस कुछ शैलियों को प्रेज़ेंटेशन में एम्बेड करने से रोकता है; उन शैलियों का पाठ इसकी अन्य शैलियों का उपयोग करता है
import-font-restricted-substituted = { $family } को प्रेज़ेंटेशन में एम्बेड नहीं किया जा सकता (लाइसेंस): मिलते-जुलते माप वाले { $by } से बदला गया
import-font-restricted-missing = { $family } को प्रेज़ेंटेशन में एम्बेड नहीं किया जा सकता (लाइसेंस): टेक्स्ट { $by } का उपयोग करता है
import-font-too-large = { $family }: फ़ाइलें डेक में एम्बेड करने के लिए बहुत बड़ी हैं; टेक्स्ट { $by } का उपयोग करता है
import-font-missing-no-fallback = { $family } नहीं मिला, न ही थीम का मुख्य फ़ॉन्ट: पाठ इंजन के डिफ़ॉल्ट फ़ॉन्ट से दिखाया जाता है
import-font-too-large-no-fallback = { $family }: फ़ाइलें डेक में एम्बेड करने के लिए बहुत बड़ी हैं, और थीम का मुख्य फ़ॉन्ट भी उपलब्ध नहीं है: पाठ इंजन के डिफ़ॉल्ट फ़ॉन्ट से दिखाया जाता है
import-font-restricted-no-fallback = { $family } को प्रेज़ेंटेशन में एम्बेड नहीं किया जा सकता (लाइसेंस), और थीम का मुख्य फ़ॉन्ट भी उपलब्ध नहीं है: पाठ इंजन के डिफ़ॉल्ट फ़ॉन्ट से दिखाया जाता है
import-warning-effect-ignored = प्रभाव आयात नहीं हुआ: { $detail }
import-warning-approximated = अनुमानित रूप में दिखाया गया: { $detail }
import-warning-unreadable-item = अपठनीय आइटम छोड़ा गया: { $detail }
import-warning-unreadable-part = अपठनीय भाग: { $detail }
import-warning-missing-part = अनुपस्थित भाग: { $detail }
import-warning-unsupported-chart = असमर्थित चार्ट: { $detail }
import-warning-unsupported-image = असमर्थित छवि फ़ॉर्मैट: { $detail }
import-warning-oversized-image = छवि आयात करने के लिए बहुत बड़ी है (पिक्सेल संख्या): { $detail }
import-warning-external-image = फ़ाइल से बाहर लिंक की गई छवि, आयात नहीं की गई: { $detail }
import-warning-unsafe-svg = SVG छवि से असुरक्षित सामग्री हटाई गई, या छवि छोड़ दी गई: { $detail }
import-warning-unknown-table-style = अज्ञात तालिका शैली: { $detail }
import-warning-unsafe-link = लिंक अस्वीकार किया गया: { $detail }
import-warning-nesting-too-deep = नेस्टिंग बहुत गहरी, काटी गई: { $detail }
import-warning-font-embedded-unreadable = एम्बेड किया गया फ़ॉन्ट अपठनीय: { $detail }
import-warning-truncated = बहुत अधिक चेतावनियाँ: बाकी छोड़ दी गईं

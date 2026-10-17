# एजेंट: सतहें, पुष्टिकरण, थ्रेड।

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
agent-confirm-edit-theme = थीम “{ $name }” का नया संस्करण सहेजें?
agent-confirm-copy-theme = बिल्ट-इन थीम “{ $name }” की संपादित प्रति सहेजें?
agent-confirm-setting = { $key } बदलें: { $old } → { $new }?

agent-not-found = एजेंट नहीं मिला: { $command }। सेटिंग [agent.presets]।
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
agent-error-pack-id = यह किसी टेम्पलेट सेट की id नहीं है: सेट और उनकी id की सूची के लिए templates बुलाएं।
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
agent-error-draft-no-agent = इस टैब का एजेंट अभी शुरू नहीं हुआ है: ड्राफ़्ट अस्वीकृत।
agent-error-unknown-agent = यह एजेंट अब कॉन्फ़िगरेशन में नहीं है।
agent-error-agent-locked = एजेंट का चुनाव परिवेश द्वारा तय है।
agent-error-agent-not-saved = एजेंट का चुनाव सहेजा नहीं जा सका।
agent-picker-title = एजेंट
agent-picker-busy = एजेंट बदलने से चल रहा टर्न रुक जाएगा।
agent-picker-locked = यह चुनाव परिवेश द्वारा तय है (OKU_SLIDES_AGENT__SELECTED)।
agent-tune-prompt = आयात की गई प्रस्तुति को बारीकी से सुधारें

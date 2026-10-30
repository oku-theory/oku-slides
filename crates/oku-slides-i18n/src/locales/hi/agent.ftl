# एजेंट: सतहें, पुष्टिकरण, थ्रेड।

agent-surface-cancel = रद्द करें
agent-surface-confirm = पुष्टि करें
agent-surface-confirm-danger = हाँ, करें
agent-surface-rewrite = पुनर्लेखन
agent-surface-pick = { $n } चुनें
agent-surface-keep = मूल रखें
agent-surface-edit = लागू करने से पहले संपादित करें
agent-surface-close = बंद करें
agent-surface-themes = थीम
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
agent-confirm-copy-pack = पैक “{ $name }” की एक प्रति सहेजें? पैक स्वयं जस का तस रहेगा।
agent-confirm-copy-edited-theme = थीम “{ $name }” की एक प्रति सहेजें? थीम स्वयं जस की तस रहेगी।
agent-confirm-delete-pack = पैक “{ $name }” हटाएं? यह पूर्ववत नहीं किया जा सकता।
agent-confirm-edit-theme = थीम “{ $name }” का नया संस्करण सहेजें?
agent-confirm-copy-theme = बिल्ट-इन थीम “{ $name }” का वैरिएंट बनाएँ? बिल्ट-इन थीम जस की तस रहेगी।
agent-confirm-setting = { $key } बदलें: { $old } → { $new }?
agent-confirm-move = प्रस्तुति को { $path } पर ले जाएँ? पुरानी फ़ाइल हटा दी जाएगी।
agent-confirm-merge = { $path } को इस प्रस्तुति में मिलाएँ? इसे पूर्ववत नहीं किया जा सकता।
agent-confirm-save-copy = प्रस्तुति की एक प्रति { $path } के रूप में सहेजें?
agent-confirm-dictation-api = आप जो बोलकर लिखवाते हैं उसे { $host } ({ $url }) पर, मॉडल { $model } के साथ और { $var } की गुप्त कुंजी से भेजें?
agent-confirm-dictation-download = { $name } उपयोग करें और अभी डाउनलोड करें ({ $size })?
agent-confirm-dictation-language = Apple डिक्टेशन उपयोग करें और उसकी भाषा अभी डाउनलोड करें?
agent-confirm-agent = कोई उत्तर न चल रहा हो तब ऐप का एजेंट { $name } ({ $command }) पर बदलें?
agent-confirm-agent-builtin = कोई उत्तर न चल रहा हो तब ऐप का एजेंट { $name } पर बदलें?
agent-confirm-agent-model = स्थानीय एजेंट का मॉडल डाउनलोड करें ({ $size })?

agent-not-found = एजेंट नहीं मिला: { $command }। सेटिंग [agent.presets]।
agent-spawn-failed = एजेंट शुरू नहीं हो सका: { $command }। सेटिंग [agent.presets]।
agent-no-http = यह एजेंट Oku Slides के टूल से कनेक्ट नहीं हो सकता।
agent-local-no-program = लोकल एजेंट केवल Oku Slides ऐप के अंदर ही चल सकता है।
agent-local-needs-download = लोकल एजेंट को पहले अपना मॉडल ({ $gigabytes } जीबी) डाउनलोड करना होगा।
agent-local-intel-mac = लोकल एजेंट के लिए Apple सिलिकॉन वाला Mac चाहिए।
agent-local-unsupported-os = लोकल एजेंट अभी इस सिस्टम पर उपलब्ध नहीं है।
agent-model-download = मॉडल डाउनलोड करें
agent-model-downloading = मॉडल डाउनलोड हो रहा है… { $percent }%
agent-model-cancel = रद्द करें
agent-model-download-failed = मॉडल डाउनलोड नहीं हो सका।
agent-model-download-integrity = डाउनलोड किया गया मॉडल क्षतिग्रस्त था और उसे हटा दिया गया है।
agent-model-download-incomplete = अधूरा डाउनलोड: { $expected } में से { $got } बाइट।
agent-model-download-cancelled = डाउनलोड रद्द किया गया।
local-agent-no-route = लोकल एजेंट समझ नहीं पाया कि क्या करना है। इसे किसी और तरह से कहकर देखें।
local-agent-tool-limit = लोकल एजेंट { $n } चरणों के बाद काम पूरा किए बिना रुक गया।
local-agent-invalid-call = लोकल एजेंट टूल का सही इस्तेमाल नहीं कर सका।
local-agent-engine-failed = लोकल एजेंट का मॉडल विफल हो गया।
local-agent-retry = फिर से कोशिश करें, या एजेंट पैनल में कोई दूसरा एजेंट चुनें।
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
agent-error-pack-id = यह किसी पैक की id नहीं है: पैक और उनकी id की सूची के लिए packs बुलाएं।
agent-error-asset-unknown = यह छवि न तो प्रस्तुति में है और न ही चिपकाई गई छवियों में: चिपकाई गई किसी छवि की आईडी का उपयोग करें।
agent-error-denied = उपयोगकर्ता ने मना कर दिया।
agent-error-no-turn = कोई टर्न चालू नहीं है: { $tool } केवल टर्न के दौरान दिखाया जाता है।
agent-error-tab-no-turn = पैक या थीम टैब तभी खोलें या दिखाएँ जब उपयोगकर्ता को उत्तर दे रहे हों।
agent-error-tab-limit = आप इस टर्न में पहले ही एक पैक या थीम टैब खोल या दिखा चुके हैं; वहीं पूरा करें, या दूसरा चुनने से पहले उपयोगकर्ता से पूछें।
agent-error-continue-in-other-tab = यह अनुरोध उसी टैब में जारी रहता है जिसे आपने खोला या दिखाया: उपयोगकर्ता से वहीं आगे बढ़ने को कहें; यह टैब फ़िलहाल कोई विजेट नहीं बदलता।
agent-error-pack-open-in-tab = इस पैक का संपादन किसी दूसरे टैब में हो रहा है: उपयोगकर्ता से पहले वहीं पूरा करने को कहें।
agent-pack-opened-in-tab = { $shown ->
   [yes] पैक { $tab } टैब में खुल गया है, जो अब दिखाया जा रहा है।
  *[no] पैक { $tab } टैब में पीछे खुल गया है: उपयोगकर्ता की विंडो में कोई दूसरा टैब दिख रहा है।
} उपयोगकर्ता से वहीं जारी रखने को कहें: उस टैब का एजेंट इस अनुरोध को नहीं जानता। यह टैब जैसा है वैसा ही रहता है।
agent-theme-opened-in-tab = { $shown ->
   [yes] थीम { $tab } टैब में खुल गई है, जो अब दिखाया जा रहा है।
  *[no] थीम { $tab } टैब में पीछे खुल गई है: उपयोगकर्ता की विंडो में कोई दूसरा टैब दिख रहा है।
} उपयोगकर्ता से वहीं जारी रखने को कहें: उस टैब का एजेंट इस अनुरोध को नहीं जानता। यह टैब जैसा है वैसा ही रहता है।
agent-error-turn-over = इस कॉल का टर्न समाप्त हो चुका है: कुछ नहीं दिखाया गया।
agent-error-no-file = प्रस्तुति की अभी कोई फ़ाइल नहीं है: पूर्ण पथ दें।
agent-error-path-relative = पथ पूर्ण होना चाहिए।
agent-error-path-parent = पथ में “..” नहीं होना चाहिए।
agent-error-setting-locked = एजेंट सेटिंग { $key } नहीं बदल सकता।
agent-error-setting = सेटिंग { $key } नहीं बदली जा सकी: उसका मान जाँचें।
agent-error-history-index = इतिहास में आगे-पीछे जाने के लिए किसी संस्करण का क्रमांक चाहिए।
agent-error-no-pack = यह प्रस्तुति कोई पैक पिन नहीं करती: pack दें (packs देखें)।
agent-error-history-not-opened = टैब ने समय पर इतिहास नहीं खोला: फिर से कहें।
agent-error-present = यहाँ प्रस्तुत करने को कुछ नहीं है: कोई स्लाइड नहीं, या टैब पैक, इतिहास या केवल-पढ़ने के मोड में है।
agent-error-locale = अज्ञात भाषा कोड। उपलब्ध भाषा का कोड या system दें।
agent-error-empty-interface = बदलने को कुछ नहीं: scheme, locale या guides दें।
agent-error-empty-dictation = बदलने को कुछ नहीं: डिक्टेशन की कम से कम एक सेटिंग दें।
agent-error-not-shown = यह टैब अपनी विंडो में दिखाया गया टैब नहीं है: उपयोगकर्ता के इसे दिखाने पर फिर से कहें।
agent-error-themes-deck-mode = थीम चयनकर्ता केवल स्लाइड संपादित करते समय खुलता है।
agent-error-packs-deck-mode = प्रस्तुति के पैक का पैनल केवल स्लाइड संपादित करते समय खुलता है।
agent-error-rewrite-text = पुनर्लेखन बिना पाठ के लौटा।
agent-error-paragraphs = पुनर्लेखन के अनुच्छेद अमान्य हैं।
agent-error-draft-too-long = ड्राफ़्ट बहुत लंबा है।
agent-error-draft-origin = ड्राफ़्ट अस्वीकृत: अज्ञात दृश्य।
agent-error-draft-no-agent = इस टैब का एजेंट अभी शुरू नहीं हुआ है: ड्राफ़्ट अस्वीकृत।
agent-error-too-many-images = ड्राफ़्ट में पहले से सभी छवियाँ हैं।
agent-error-image-not-base64 = चिपकाई गई छवि पूरी नहीं पहुँची।
agent-error-image-gone = चिपकाई गई एक छवि अब उपलब्ध नहीं है: उसे फिर से चिपकाएँ।
agent-error-unknown-agent = यह एजेंट अब कॉन्फ़िगरेशन में नहीं है।
agent-error-agent-locked = एजेंट का चुनाव परिवेश द्वारा तय है।
agent-error-agent-not-saved = एजेंट का चुनाव सहेजा नहीं जा सका।
agent-error-unknown-model = यह एजेंट वह मॉडल नहीं देता।
agent-error-model-not-saved = मॉडल का चुनाव सहेजा नहीं जा सका।
voice-error-no-dictation = यहाँ डिक्टेशन उपलब्ध नहीं है
voice-error-busy = डिक्टेशन चल रहा है: रुकने पर फिर से कोशिश करें
voice-error-unknown-model = अज्ञात डिक्टेशन मॉडल
voice-error-unavailable = यह मॉडल चुनी गई भाषा का समर्थन नहीं करता
voice-error-installed = यह मॉडल पहले से इंस्टॉल है
voice-error-not-downloadable = यह मॉडल डाउनलोड नहीं होता
voice-error-nothing-to-cancel = कोई डाउनलोड नहीं चल रहा
voice-error-bad-silence = मौन 0.5 से 10 सेकंड तक का होना चाहिए
voice-error-bad-language = अज्ञात डिक्टेशन भाषा
voice-error-bad-url = API पता https:// से शुरू होना चाहिए (http:// केवल इस कंप्यूटर के लिए), बिना उपयोगकर्ता नाम, पैरामीटर या विशेष वर्णों के
voice-error-bad-key-env = कुंजी का वेरिएबल OPENAI_API_KEY जैसा नाम होना चाहिए: बड़े अक्षर, अंक और _
voice-error-bad-api-model = API मॉडल के नाम में केवल अक्षर, अंक और . _ : / - हो सकते हैं; Azure डिप्लॉयमेंट के नाम में केवल अक्षर, अंक और . _ -
voice-error-bad-api-version = Azure API संस्करण में केवल अक्षर, अंक, . और - हो सकते हैं, अधिकतम 32 वर्ण, जैसे 2025-03-01-preview
voice-error-bad-api-headers = हेडर Name: value; Name: $VARIABLE के रूप में लिखे जाते हैं, अधिकतम 8। Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection और दोहराव अस्वीकार किए जाते हैं
voice-error-api-reserved = API का पता, कुंजी चर, Azure संस्करण और हेडर केवल उपयोगकर्ता सेट करता है: show_dictation से बॉक्स खोलें और उनसे कहें
voice-error-changed = प्रश्न के बाद सेटिंग्स बदल गई हैं: फिर से पूछें
voice-error-nothing-to-download = इस एजेंट के लिए डाउनलोड करने को कुछ नहीं है
agent-tune-prompt = आयात की गई प्रस्तुति को बारीकी से सुधारें
agent-settings-unknown-selected = सेटिंग agent.selected = { $selected } [agent.presets] के किसी एजेंट का नाम नहीं है (ज्ञात: { $known })
agent-settings-preset-launch = एजेंट [agent.presets.{ $id }] को command या builtin में से ठीक एक चाहिए

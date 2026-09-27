# एजेंट: सतहें, पुष्टिकरण, थ्रेड।

agent-instructions = आप उस उपयोगकर्ता के लिए Oku Slides प्रस्तुति संपादित करते हैं जो आपसे बात करता है। केवल oku-slides टूल के माध्यम से कार्य करें। बदलने से पहले deck_outline, slide और render_slide के साथ डेक पढ़ें; इसे apply के साथ बदलें (हर बदलाव पूर्ववत किया जा सकता है)। जब निर्णय उपयोगकर्ता का हो, तो ask_choice, ask_confirm या ask_rewrite के साथ पूछें; show_variants के साथ लेआउट और show_params के साथ लाइव सेटिंग्स प्रस्तुत करें। संक्षेप में, उपयोगकर्ता की भाषा में उत्तर दें। प्रत्येक संदेश एक <oku-slides-context> ब्लॉक से शुरू होता है जो बताता है कि उपयोगकर्ता क्या देख रहा है।

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
agent-confirm-open-dirty = { $path } खोलें? न सहेजे गए बदलाव खो जाएँगे।
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

# অ্যাপের ইন্টারফেস (React)। ui/src/i18n.ts «messages» অনুরোধের মাধ্যমে এটি লোড করে।
ui-loading = লোড হচ্ছে…

## ডকুমেন্ট

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · টেমপ্লেট
ui-title-rename = প্রেজেন্টেশনের নাম বদলান
ui-title-field = প্রেজেন্টেশনের শিরোনাম
ui-status-unsaved = অসংরক্ষিত পরিবর্তন
ui-status-saved = সংরক্ষিত
ui-status-never-saved = এখনও সংরক্ষিত হয়নি
ui-caption-templates = { $count ->
    [one] { $count }টি টেমপ্লেট
   *[other] { $count }টি টেমপ্লেট
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count }টি স্লাইড
   *[other] { $count }টি স্লাইড
} · { $status }
ui-template-default-name = টেমপ্লেট
ui-template-variant-name = { $name } (ভ্যারিয়েন্ট)
ui-confirm-quit-templates = পরিবর্তিত টেমপ্লেট সংরক্ষণ না করেই বেরিয়ে যাবেন?

## বিজ্ঞপ্তি

ui-toast-templates-saved = টেমপ্লেট সংরক্ষিত: { $name }
ui-toast-moved = উপস্থাপনা সরানো হয়েছে
ui-toast-copied = অনুলিপি সংরক্ষিত
ui-toast-select-zone = বসানোর জন্য একটি এলাকা বেছে নিন: “{ $text }”
ui-toast-field-gone = আপনি যে ঘরটি সম্পাদনা করছিলেন সেটি আর নেই: আপনার লেখা হারিয়ে গেছে।
ui-toast-edit-replaced = আপনি সম্পাদনা করার সময় এই লেখাটি বদলে গেছে: আপনার সংস্করণ সেটির জায়গা নিয়েছে। পূর্বাবস্থায় ফেরান অন্যটি ফিরিয়ে আনে।

## টুলবার

ui-toolbar-undo = পূর্বাবস্থায় ফেরান
ui-toolbar-undo-label = পূর্বাবস্থায় ফেরান: { $label }
ui-toolbar-redo = আবার করুন
ui-toolbar-redo-label = আবার করুন: { $label }
ui-toolbar-grid-hide = গ্রিড লুকান
ui-toolbar-grid-show = গ্রিড দেখান
ui-toolbar-ghosts-hide = গাইড লুকান
ui-toolbar-ghosts-show = গাইড দেখান
ui-divider-pile = স্লাইড স্তূপের প্রস্থ
ui-divider-notes = এজেন্ট থ্রেডের প্রস্থ
ui-divider-code = কোডের আকার
ui-divider-band = সেটিংসের উচ্চতা
ui-toolbar-theme = থিম
ui-toolbar-theme-embedded = ফাইলের থিম
ui-toolbar-save-copy = একটি কপি সংরক্ষণ করুন
ui-toolbar-finish = সম্পন্ন
ui-menu-workshop = কর্মশালা
ui-toolbar-open = খুলুন
ui-merge = PDF-এর সঙ্গে একত্র করুন…
ui-menu = মেনু
ui-menu-move = সরান…
ui-language = ভাষা
ui-language-system = সিস্টেমের ভাষা
ui-scheme = চেহারা
ui-scheme-system = সিস্টেম
ui-scheme-light = হালকা
ui-scheme-dark = গাঢ়
ui-toolbar-add-label = স্লাইডে যোগ করুন
ui-add-kind = { $kind ->
    [card] কার্ড
    [shape] আকৃতি
    [chart] চার্ট
    [table] টেবিল
   *[text] টেক্সট
}
ui-add-widget = প্যাক উইজেট
ui-toolbar-remove-template = টেমপ্লেট মুছুন
ui-toolbar-remove-slide = স্লাইড মুছুন
ui-toolbar-remove-hint = স্লাইড { $n } · ⌘Z দিয়ে পূর্বাবস্থায় ফেরান

## পূর্বাবস্থার লেবেল

ui-undo-add = { $kind ->
    [card] কার্ড যোগ করুন
    [shape] আকৃতি যোগ করুন
    [chart] চার্ট যোগ করুন
    [table] টেবিল যোগ করুন
   *[text] টেক্সট যোগ করুন
}
ui-undo-add-widget = “{ $name }” যোগ করুন
ui-undo-new-template = নতুন টেমপ্লেট: { $name }
ui-undo-new-slide = নতুন স্লাইড: { $name }
ui-undo-templates = টেমপ্লেট: { $name }
ui-undo-move-template = টেমপ্লেট সরান
ui-undo-move-slide = স্লাইড সরান
ui-undo-remove-slide = স্লাইড মুছুন
ui-undo-remove = মুছুন
ui-undo-paste = পেস্ট
ui-undo-place-on-grid = গ্রিডে বসান
ui-undo-move-on-grid = গ্রিডে সরান
ui-undo-resize-on-grid = গ্রিডে আকার বদলান
ui-undo-change-anchor = অ্যাঙ্করিং বদলান
ui-undo-move = সরান
ui-undo-resize = আকার বদলান
ui-undo-edit-slide = স্লাইড সম্পাদনা করুন
ui-undo-dictation = ডিক্টেশন
ui-undo-rename-set = সেটের নাম বদলান
ui-undo-rename-deck = প্রেজেন্টেশনের নাম বদলান
ui-undo-chart-kind = চার্টের ধরন বদলান
ui-undo-chart-data = ডেটা সম্পাদনা করুন
ui-undo-chart-color = সিরিজে রং দিন
ui-undo-chart-stack = { $stacked ->
    [yes] সিরিজগুলো স্তূপ করুন
   *[no] সিরিজগুলো আলাদা করুন
}

## নতুন উপাদান

ui-new-text = নতুন টেক্সট
ui-new-card-title = কার্ডের শিরোনাম
ui-new-card-detail = বিস্তারিত

## স্লাইডের তালিকা

ui-pile-slides = স্লাইড
ui-pile-templates = টেমপ্লেট
ui-pile-add-slide = + নতুন স্লাইড
ui-pile-add-template = + নতুন টেমপ্লেট
ui-pile-drag-hint = সরাতে টেনে আনুন · ⌥↑ ⌥↓
ui-slide-alt = স্লাইড { $n }

## মঞ্চ

ui-stage-rendering = রেন্ডার হচ্ছে…
ui-stage-elements = স্লাইডের উপাদান
ui-stage-fields = স্লাইডের ঘর
ui-stage-edit-text = টেক্সট সম্পাদনা করুন
ui-stage-hint-plain = পরেরটিতে যেতে Enter বা Tab · বাতিল করতে Esc
ui-stage-hint-rich = পরেরটিতে যেতে Tab · নিশ্চিত করতে ⌘ Enter · বুলেটের জন্য “- ” · ⌘B বোল্ড, ⌘I ইটালিক, ⌘K লিঙ্ক
ui-field = { $kind ->
    [title] শিরোনাম
    [subtitle] উপশিরোনাম
    [kicker] কিকার
    [text] টেক্সট
    [body] মূল অংশ
    [author] লেখক
    [date] তারিখ
    [organization] প্রতিষ্ঠান
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] টেক্সট
    [card] কার্ড
    [image] ছবি
    [shape] আকৃতি
    [group] গ্রুপ
   *[other] { $kind }
}
ui-grid-cell = কলাম { $cols } · সারি { $rows }
ui-ghost-zone = অঞ্চল

## স্লাইড নোট

ui-notes = স্লাইড টুল
ui-stepper-less = { $label }: একটি কম
ui-stepper-more = { $label }: একটি বেশি
ui-list-item = আইটেম
ui-list-add = { $item } যোগ করুন
ui-list-remove = { $item } সরান
ui-list-move = { $item } সরিয়ে নিন
ui-list-full = তালিকা পূর্ণ ({ $max })
ui-undo-list-move = { $item } সরিয়ে নিন
ui-agent-color-other = অন্য রং
ui-agent-thread = এজেন্ট
ui-agent-new-conversation = নতুন কথোপকথন
ui-agent-undo-turn = এই পালাটি পূর্বাবস্থায় ফেরান
ui-agent-detach = এজেন্টকে আলাদা উইন্ডোতে খুলুন
ui-agent-attach = এজেন্টকে মূল উইন্ডোতে ফিরিয়ে আনুন
ui-agent-window-empty = এই ট্যাবের জন্য কোনো এজেন্ট নেই
ui-agent-kept-in-draft = লেখার জায়গা নেই: লেখাটি খসড়ায় রাখা হয়েছে
ui-agent-busy-elsewhere = এজেন্ট { $place }-এ উত্তর দিচ্ছে…
ui-agent-scope-slides = স্লাইড
ui-agent-scope-atelier = কর্মশালা
ui-draft = খসড়া
ui-draft-insert = বসান
ui-draft-insert-title = যেমন আছে তেমন বসান (⌘⇧↩)
ui-draft-send = এজেন্টকে পাঠান
ui-draft-send-title = এজেন্টকে পাঠান (⌘↩)
ui-draft-discard = বাতিল
ui-agent-state-idle = এজেন্ট
ui-agent-state-starting = এজেন্ট চালু হচ্ছে…
ui-agent-state-working = এজেন্ট কাজ করছে…
ui-agent-dock = চলমান
ui-agent-dock-settings = সেটিংস
ui-agent-jump = ↓ বর্তমান পালা
ui-agent-tool-input = আর্গুমেন্ট
ui-agent-tool-output = ফলাফল
ui-agent-sheet = এজেন্টের প্রশ্ন
ui-agent-sheet-keys = 1–4 বেছে নিন · Enter নিশ্চিত · E সম্পাদনা · Esc বাতিল
ui-agent-variants = বিকল্পগুলো
ui-agent-drawer-keys = ক্লিক বা 1–4 · Esc বন্ধ করে
ui-agent-confirm = এজেন্টের নিশ্চিতকরণ

## ডিক্টেশন

ui-dictation = ডিক্টেশন
ui-dictation-start = বলে লিখুন
ui-dictation-start-title = বলে লিখুন (⌘D)
ui-dictation-type = এজেন্টকে লিখুন
ui-dictation-type-title = এজেন্টকে লিখুন (⌘K)
ui-dictation-agent-stop = এজেন্ট থামান (⌘⇧D বা Esc)
ui-dictation-agent-working = এজেন্ট কাজ করছে…
ui-dictation-agent-working-hint = ⌘⇧D বা Esc থামায়
ui-dictation-finish = ডিক্টেশন শেষ করুন
ui-dictation-finish-title = শেষ করুন (⌘D)
ui-dictation-cancel = বাতিল
ui-dictation-preparing = ডিক্টেশন প্রস্তুত হচ্ছে…
ui-dictation-heard-nothing = কিছু শোনা যায়নি: মাইক্রোফোনের একটু কাছে এসে বলুন
ui-dictation-to-cursor = কার্সরের জায়গায়
ui-dictation-to-draft = খসড়ায়
ui-dictation-to-field = { $field } প্রতিস্থাপন করুন
ui-dictation-to-block = নির্বাচিত ব্লকের পরে
ui-dictation-new-block = নতুন টেক্সট ব্লক
ui-dictation-select-zone = একটি এলাকা বেছে নিন
ui-dictation-downloading = { $name } মডেল ডাউনলোড হচ্ছে
ui-dictation-download-progress = { $percent }% · { $total }-এর মধ্যে { $received } · শুধু একবার
ui-dictation-listening = শোনা হচ্ছে · { $time }
ui-dictation-transcribing = শেষ বাক্যটি লেখা হচ্ছে…
ui-dictation-stop-in = { $duration } পরে থামবে
ui-dictation-speak = বলুন, টেক্সট এখানে দেখা যাবে…
ui-dictation-auto-stop = { $duration } নীরবতার পরে থামে
ui-dictation-to-download = { $size } ডাউনলোড করতে হবে
ui-dictation-settings = সেটিংস
ui-dictation-model-title = শনাক্তকরণ মডেল
ui-dictation-model-intro = Whisper, এই কম্পিউটারেই (CTranslate2)। প্রতিটি মডেল একাধিক ভাষা বোঝে।
ui-dictation-note-turbo = প্রস্তাবিত, দ্রুত ও নির্ভুল
ui-dictation-note-large = সবচেয়ে নির্ভুল, তবে ধীর
ui-dictation-note-base = হালকা, ছোট মেশিনের জন্য
ui-dictation-installed = ইনস্টল করা আছে
ui-dictation-not-installed = ডাউনলোড করতে হবে
ui-dictation-language = কথ্য ভাষা
ui-dictation-language-auto = স্বয়ংক্রিয় শনাক্তকরণ
ui-dictation-note-apple = সবচেয়ে দ্রুত, সিস্টেমে অন্তর্নির্মিত
ui-dictation-model-unavailable = ভাষা সমর্থিত নয়
ui-dictation-model-intro-apple = Apple, সিস্টেমে অন্তর্নির্মিত, অথবা এই কম্পিউটারে Whisper (CTranslate2)। সবগুলোই একাধিক ভাষা বোঝে।
ui-dictation-language-auto-resolved = স্বয়ংক্রিয় ({ $language })
ui-dictation-to-download-language = ভাষা ডাউনলোড করতে হবে
ui-dictation-auto-stop-label = এতক্ষণ নীরবতার পরে নিজে থেকে থামুন
ui-dictation-silence = নীরবতার সময়কাল
ui-dictation-phrases-note = আপনি বলার সময় শুধু কথিত অংশগুলো, বাক্য ধরে ধরে, লেখা হয়।
ui-dictation-download-now = এখনই ডাউনলোড করুন ({ $size })
ui-dictation-download-language = ভাষাটি ডাউনলোড করুন
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } সে.
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## টেমপ্লেট নির্বাচক

ui-picker-new-slide = নতুন স্লাইড
ui-picker-new-template = নতুন টেমপ্লেট, যার ভিত্তি…
ui-picker-set = টেমপ্লেট
ui-picker-set-mine = { $name } · আমার
ui-picker-set-received = { $name } · প্রাপ্ত
ui-picker-keys = ব্রাউজ করতে ← → · যোগ করতে Enter · বন্ধ করতে Esc
ui-picker-close = বন্ধ করুন
ui-picker-previous = আগের টেমপ্লেট
ui-picker-next = পরের টেমপ্লেট
ui-picker-list = স্লাইড টেমপ্লেট

## চার্ট

ui-chart-panel = চার্ট
ui-chart-kinds = চার্টের ধরন
ui-chart-kind = { $kind ->
    [line] লাইন
    [pie] পাই
    [scatter] বিক্ষেপ
    [radar] রাডার
    [waterfall] ওয়াটারফল
   *[bar] বার
}
ui-chart-no-data = এই চার্টে কোনো ডেটা টেবিল নেই।
ui-chart-options = চার্টের বিকল্প
ui-chart-stack = স্তূপীকৃত
ui-chart-waterfall-hint = মোটের বার দেখাতে «মোট» টিক দিন।
ui-chart-colors = সিরিজের রং
ui-chart-color-of = { $name }-এর রং
ui-chart-color-auto = স্বয়ংক্রিয়
ui-chart-color-pick = রং { $n }
ui-chart-column-name = কলাম { $col }-এর নাম
ui-chart-remove-column = কলাম মুছুন
ui-chart-cell = সারি { $row }, কলাম { $col }
ui-chart-remove-row = সারি মুছুন
ui-chart-add-row = সারি
ui-chart-add-column = কলাম
ui-chart-series = সিরিজ { $n }

## টেবিল

ui-table-panel = টেবিল
ui-table-options = টেবিলের বিকল্প
ui-table-header-row = শিরোনাম সারি
ui-table-header-column = প্রথম কলাম শিরোনাম হিসেবে
ui-table-striped = ডোরাকাটা সারি
ui-table-alignment = কলামের বিন্যাস
ui-table-align-of = { $name }-এর বিন্যাস
ui-table-align = { $align ->
    [start] শুরু
    [center] মাঝ
    [end] শেষ
   *[auto] স্বয়ংক্রিয়
}
ui-table-column = কলাম { $n }
ui-table-no-data = এই টেবিলে কোনো ডেটা নেই।
ui-undo-table-option = টেবিলের বিকল্প
ui-undo-table-align = কলাম সারিবদ্ধ করুন
ui-undo-table-data = টেবিল সম্পাদনা করুন
ui-show-as = এভাবে দেখান
ui-show-as-table = টেবিল হিসেবে দেখান
ui-show-as-chart = চার্ট হিসেবে দেখান
ui-undo-show-as-table = টেবিল হিসেবে দেখান
ui-undo-show-as-chart = চার্ট হিসেবে দেখান

## ফাইল

ui-file-open-prompt = যে ফাইল খুলবেন তার পাথ (.oku-slides.pdf বা .json)
ui-file-save-prompt = এই হিসেবে সংরক্ষণ করুন (পূর্ণ পাথ)

## ট্যাব

ui-tabs = খোলা উপস্থাপনা
ui-tab-new = নতুন ট্যাব
ui-tab-close = ট্যাব বন্ধ করুন
ui-tab-unsaved = অসংরক্ষিত পরিবর্তন
ui-tab-templates = টেমপ্লেট
ui-tab-move-new-window = নতুন উইন্ডোতে সরান
ui-tab-move-to = উইন্ডো { $n }-এ সরান
ui-tab-close-others = অন্য ট্যাবগুলি বন্ধ করুন
ui-close-cancel = বাতিল
ui-disk-replaced-question = ডিস্কে { $file } অন্য একটি উপস্থাপনা দিয়ে প্রতিস্থাপিত হয়েছে।
ui-disk-replaced-overwrite = ওভাররাইট করুন
ui-disk-replaced-save-as = এভাবে সংরক্ষণ করুন…
ui-same-deck-question = { $file } হলো “{ $title }” ট্যাবে খোলা উপস্থাপনার একটি সংস্করণ।
ui-same-deck-merge = ওই ট্যাবে একত্র করুন
ui-same-deck-apart = আলাদাভাবে খুলুন
ui-history = ইতিহাস
ui-history-version = { $date }-এর সংস্করণ
ui-history-restore = পুনরুদ্ধার করুন
ui-history-back = ফিরে যান
ui-history-anonymous = বেনামী
ui-history-slider = উপস্থাপনার সংস্করণ
ui-history-removed-since = তারপর থেকে মুছে ফেলা হয়েছে
ui-history-changed-since = তারপর থেকে পরিবর্তিত
ui-history-added-since = তারপর থেকে যোগ করা স্লাইড: { $count }
ui-help = সহায়িকা
ui-help-caption = স্লাইড { $index } / { $count } · শুধু পড়ার জন্য
ui-slide-previous = আগের স্লাইড
ui-slide-next = পরের স্লাইড
ui-present = উপস্থাপনা
ui-present-slide = স্লাইড { $index } / { $count }
ui-present-screen = পূর্ণ স্ক্রিনে উপস্থাপন করুন
ui-present-window = উইন্ডোতে উপস্থাপন করুন
ui-toolbar-params-hide = সেটিংস লুকান
ui-toolbar-params-show = সেটিংস দেখান
ui-help-close = সহায়িকা বন্ধ করুন
ui-help-search = সহায়িকায় খুঁজুন
ui-help-search-placeholder = খুঁজুন…
ui-help-search-empty = কোনো ফল নেই
ui-help-search-count = { $count }টি ফল
ui-pack-notice-close = বন্ধ করুন
ui-widgets-files = উইজেট ফাইল
ui-widgets-common = সাধারণ
ui-widgets-new = নতুন উইজেট
ui-widgets-name = উইজেটের নাম (ছোট হাতের অক্ষর, অঙ্ক ও হাইফেন)
ui-widgets-name-taken = এই নামটি ইতিমধ্যে নেওয়া হয়েছে
ui-widgets-name-letter = নামটি একটি অক্ষর দিয়ে শুরু হতে হবে
ui-widgets-valid = বৈধ
ui-widgets-errors = ত্রুটি
ui-widgets-empty = এই প্যাকে এখনও কোনো উইজেট নেই।
ui-widgets-preview = উইজেটের প্রিভিউ
ui-widgets-no-preview = প্রিভিউ নেই
ui-widgets-field-bound = উপস্থাপনার সঙ্গে যুক্ত: এখানে শুধু পড়া যায়
ui-widgets-source = সোর্স কোড
ui-widgets-unsaved = সংরক্ষিত হয়নি
ui-widgets-in-error = ত্রুটিযুক্ত উইজেট
ui-widgets-none-in-error = কোনো উইজেটে ত্রুটি নেই
ui-widgets-section = উইজেট
ui-widgets-open = উইজেট { $name } খুলুন
ui-widgets-delete-named = { $name } মুছুন
ui-widgets-confirm-delete = উইজেট “{ $name }” মুছবেন?
ui-widgets-rename-hint = নাম বদলাতে ডাবল-ক্লিক করুন বা F2 চাপুন
ui-widgets-tabs = টেমপ্লেট ট্যাব
ui-widgets-templates-tab = টেমপ্লেট
ui-theme-tab = থিম
ui-theme-picker = থিম
ui-theme-name = থিমের নাম
ui-theme-unsaved = থিমের পরিবর্তন সংরক্ষিত হয়নি
ui-widgets-close-tab = { $name } বন্ধ করুন
ui-widgets-removed-by-agent = এজেন্ট উইজেট { $name } মুছে ফেলেছে।

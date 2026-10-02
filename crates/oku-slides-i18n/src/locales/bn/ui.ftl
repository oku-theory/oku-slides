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

## টুলবার

ui-toolbar-undo = পূর্বাবস্থায় ফেরান
ui-toolbar-undo-label = পূর্বাবস্থায় ফেরান: { $label }
ui-toolbar-redo = আবার করুন
ui-toolbar-redo-label = আবার করুন: { $label }
ui-toolbar-grid-hide = গ্রিড লুকান
ui-toolbar-grid-show = গ্রিড দেখান
ui-toolbar-ghosts-hide = ঘোস্ট লুকান
ui-toolbar-ghosts-show = ঘোস্ট দেখান
ui-toolbar-theme = থিম
ui-toolbar-theme-default = ডিফল্ট
ui-toolbar-save-copy = একটি কপি সংরক্ষণ করুন
ui-toolbar-finish = সম্পন্ন
ui-toolbar-templates = টেমপ্লেট
ui-toolbar-open = খুলুন
ui-merge = PDF-এর সঙ্গে একত্র করুন…
ui-menu = মেনু
ui-menu-move = সরান…
ui-language = ভাষা
ui-language-system = সিস্টেমের ভাষা
ui-toolbar-add-label = স্লাইডে যোগ করুন
ui-add-kind = { $kind ->
    [card] কার্ড
    [shape] আকৃতি
    [chart] চার্ট
   *[text] টেক্সট
}
ui-toolbar-remove-template = টেমপ্লেট মুছুন
ui-toolbar-remove-slide = স্লাইড মুছুন
ui-toolbar-remove-hint = স্লাইড { $n } · ⌘Z দিয়ে পূর্বাবস্থায় ফেরান

## পূর্বাবস্থার লেবেল

ui-undo-add = { $kind ->
    [card] কার্ড যোগ করুন
    [shape] আকৃতি যোগ করুন
    [chart] চার্ট যোগ করুন
   *[text] টেক্সট যোগ করুন
}
ui-undo-new-template = নতুন টেমপ্লেট: { $name }
ui-undo-new-slide = নতুন স্লাইড: { $name }
ui-undo-grid = গ্রিড { $columns } × { $rows }
ui-undo-templates = টেমপ্লেট: { $name }
ui-undo-move-template = টেমপ্লেট সরান
ui-undo-move-slide = স্লাইড সরান
ui-undo-remove-slide = স্লাইড মুছুন
ui-undo-remove = মুছুন
ui-undo-cut = কাট
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
ui-undo-theme = থিম বদলান
ui-undo-rename-template = টেমপ্লেটের নাম বদলান
ui-undo-describe-template = টেমপ্লেটের বিবরণ দিন
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
ui-agent-color-other = অন্য রং
ui-agent-thread = এজেন্ট
ui-agent-new-conversation = নতুন কথোপকথন
ui-agent-tools = { $count }টি টুল
ui-draft = খসড়া
ui-draft-insert = বসান
ui-draft-insert-title = যেমন আছে তেমন বসান (⌘⇧↩)
ui-draft-send = এজেন্টকে পাঠান
ui-draft-send-title = এজেন্টকে পাঠান (⌘↩)
ui-draft-discard = বাতিল
ui-agent-state-idle = এজেন্ট
ui-agent-state-starting = এজেন্ট চালু হচ্ছে…
ui-agent-state-working = এজেন্ট কাজ করছে…
ui-agent-stopped = থামানো হয়েছে।
ui-agent-ended = পালা শেষ ({ $stop })।
ui-agent-stop-max-tokens = এজেন্ট দৈর্ঘ্যের সীমায় পৌঁছেছে।
ui-agent-stop-max-turns = এজেন্ট ধাপের সীমায় পৌঁছেছে।
ui-agent-stop-refusal = এজেন্ট প্রত্যাখ্যান করেছে।
ui-agent-sheet = এজেন্টের প্রশ্ন
ui-agent-sheet-keys = 1–4 বেছে নিন · Enter নিশ্চিত · E সম্পাদনা · Esc বাতিল
ui-agent-variants = বিকল্পগুলো
ui-agent-drawer-keys = ক্লিক বা 1–4 · Esc বন্ধ করে
ui-agent-confirm = এজেন্টের নিশ্চিতকরণ

## ডিক্টেশন

ui-dictation = ডিক্টেশন
ui-dictation-start = বলে লিখুন
ui-dictation-start-title = বলে লিখুন (⌘D)
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
ui-dictation-settings = ডিক্টেশন সেটিংস
ui-dictation-model-title = শনাক্তকরণ মডেল
ui-dictation-model-intro = Whisper, এই কম্পিউটারেই (CTranslate2)। প্রতিটি মডেল একাধিক ভাষা বোঝে।
ui-dictation-note-turbo = প্রস্তাবিত, দ্রুত ও নির্ভুল
ui-dictation-note-large = সবচেয়ে নির্ভুল, তবে ধীর
ui-dictation-note-base = হালকা, ছোট মেশিনের জন্য
ui-dictation-installed = ইনস্টল করা আছে
ui-dictation-not-installed = ডাউনলোড করতে হবে
ui-dictation-language = কথ্য ভাষা
ui-dictation-language-auto = স্বয়ংক্রিয় শনাক্তকরণ
ui-dictation-auto-stop-label = এতক্ষণ নীরবতার পরে নিজে থেকে থামুন
ui-dictation-silence = নীরবতার সময়কাল
ui-dictation-phrases-note = আপনি বলার সময় শুধু কথিত অংশগুলো, বাক্য ধরে ধরে, লেখা হয়।
ui-dictation-download-now = এখনই ডাউনলোড করুন ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } সে.
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## টেমপ্লেট নির্বাচক

ui-picker-new-slide = নতুন স্লাইড
ui-picker-new-template = নতুন টেমপ্লেট, যার ভিত্তি…
ui-picker-set = টেমপ্লেট
ui-picker-set-mine = { $name } · আমার
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
ui-pack-notice-close = বন্ধ করুন

# অ্যাপের ইন্টারফেস (React)। ui/src/i18n.ts «messages» অনুরোধের মাধ্যমে এটি লোড করে।
ui-loading = লোড হচ্ছে…

## ডকুমেন্ট

ui-brand = Écho
ui-brand-templates = Écho · টেমপ্লেট
ui-title-rename = প্রেজেন্টেশনের নাম বদলান
ui-title-field = প্রেজেন্টেশনের শিরোনাম
ui-status-unsaved = অসংরক্ষিত পরিবর্তন
ui-status-saved = সংরক্ষিত
ui-status-never-saved = এখনও সংরক্ষিত হয়নি
ui-status-builtin = বিল্ট-ইন: সংরক্ষণ করলে একটি কপি তৈরি হবে
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

ui-toast-saved = আপনার প্রেজেন্টেশন সংরক্ষিত হয়েছে
ui-toast-templates-saved = টেমপ্লেট সংরক্ষিত: { $name }
ui-toast-select-zone = বসানোর জন্য একটি এলাকা বেছে নিন: “{ $text }”

## টুলবার

ui-toolbar-undo = পূর্বাবস্থায় ফেরান
ui-toolbar-undo-label = পূর্বাবস্থায় ফেরান: { $label }
ui-toolbar-redo = আবার করুন
ui-toolbar-redo-label = আবার করুন: { $label }
ui-toolbar-grid-hide = গ্রিড লুকান
ui-toolbar-grid-show = গ্রিড দেখান
ui-toolbar-grid-hide-title = গ্রিড লুকান (⌘')
ui-toolbar-grid-show-title = গ্রিড দেখান (⌘')
ui-toolbar-theme = থিম
ui-toolbar-theme-default = ডিফল্ট
ui-toolbar-save-copy = একটি কপি সংরক্ষণ করুন
ui-toolbar-save = সংরক্ষণ করুন
ui-toolbar-finish = সম্পন্ন
ui-toolbar-templates = টেমপ্লেট
ui-toolbar-templates-title = স্লাইড টেমপ্লেট তৈরি ও সম্পাদনা করুন
ui-toolbar-open = খুলুন

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
ui-undo-place-on-grid = গ্রিডে বসান
ui-undo-move-on-grid = গ্রিডে সরান
ui-undo-resize-on-grid = গ্রিডে আকার বদলান
ui-undo-change-anchor = অ্যাঙ্করিং বদলান
ui-undo-move = সরান
ui-undo-resize = আকার বদলান
ui-undo-edit-text = টেক্সট সম্পাদনা করুন
ui-undo-dictation = ডিক্টেশন
ui-undo-rename-set = সেটের নাম বদলান
ui-undo-rename-deck = প্রেজেন্টেশনের নাম বদলান
ui-undo-theme = থিম বদলান
ui-undo-rename-template = টেমপ্লেটের নাম বদলান
ui-undo-describe-template = টেমপ্লেটের বিবরণ দিন
ui-undo-chart-kind = চার্টের ধরন বদলান
ui-undo-chart-data = ডেটা সম্পাদনা করুন

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
ui-stage-edit-text = টেক্সট সম্পাদনা করুন
ui-stage-hint-field = নিশ্চিত করতে Enter · বাতিল করতে Esc
ui-stage-hint-block = নিশ্চিত করতে ⌘ Enter · প্রতি অনুচ্ছেদে এক লাইন, বুলেটের জন্য “- ”
ui-field = { $kind ->
    [title] শিরোনাম
    [subtitle] উপশিরোনাম
    [kicker] কিকার
    [text] টেক্সট
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

## স্লাইড নোট

ui-notes = স্লাইড টুল
ui-note-template = এই টেমপ্লেট
ui-note-template-name = নাম
ui-note-template-description = বিবরণ
ui-note-add-label = স্লাইডে যোগ করুন
ui-note-add = যোগ করুন
ui-add-kind = { $kind ->
    [card] কার্ড
    [shape] আকৃতি
    [chart] চার্ট
   *[text] টেক্সট
}
ui-note-grid-label = টেমপ্লেটের গ্রিড
ui-note-grid = গ্রিড
ui-note-grid-columns = কলাম
ui-note-grid-rows = সারি
ui-stepper-less = { $label }: একটি কম
ui-stepper-more = { $label }: একটি বেশি
ui-note-remove-template = টেমপ্লেট মুছুন
ui-note-remove-slide = স্লাইড মুছুন
ui-note-remove-hint = স্লাইড { $n } · ⌘Z দিয়ে পূর্বাবস্থায় ফেরান
ui-note-tip = টিপ
ui-note-tip-text = ডিক্টেশনের জন্য ⌘D। গ্রিড দেখাতে ⌘'। টেক্সট সম্পাদনা করতে ডাবল-ক্লিক করুন।

## ডিক্টেশন

ui-dictation = ডিক্টেশন
ui-dictation-start = বলে লিখুন
ui-dictation-start-title = বলে লিখুন (⌘D)
ui-dictation-finish = ডিক্টেশন শেষ করুন
ui-dictation-finish-title = শেষ করুন (⌘D)
ui-dictation-cancel = বাতিল
ui-dictation-preparing = ডিক্টেশন প্রস্তুত হচ্ছে…
ui-dictation-heard-nothing = কিছু শোনা যায়নি: মাইক্রোফোনের একটু কাছে এসে বলুন
ui-dictation-to-cursor = কার্সরের জায়গায়
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
   *[bar] বার
}
ui-chart-no-data = এই চার্টে কোনো ডেটা টেবিল নেই।
ui-chart-column-name = কলাম { $col }-এর নাম
ui-chart-remove-column = কলাম মুছুন
ui-chart-cell = সারি { $row }, কলাম { $col }
ui-chart-remove-row = সারি মুছুন
ui-chart-add-row = + সারি
ui-chart-add-column = + কলাম
ui-chart-series = সিরিজ { $n }

## ফাইল

ui-file-open-prompt = যে ফাইল খুলবেন তার পাথ (.echo.pdf বা .json)
ui-file-save-prompt = এই হিসেবে সংরক্ষণ করুন (পূর্ণ পাথ)

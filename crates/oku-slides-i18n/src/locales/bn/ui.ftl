# অ্যাপের ইন্টারফেস (React)। ui/src/i18n.ts «messages» অনুরোধের মাধ্যমে এটি লোড করে।
ui-loading = লোড হচ্ছে…

## ডকুমেন্ট

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · লেআউট প্যাক
   *[other] { $name } · উইজেট প্যাক
}
ui-title-rename = প্রেজেন্টেশনের নাম বদলান
ui-title-field = প্রেজেন্টেশনের শিরোনাম
ui-status-unsaved = অসংরক্ষিত পরিবর্তন
ui-status-saved = সংরক্ষিত
ui-status-never-saved = এখনও সংরক্ষিত হয়নি
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count }টি লেআউট
       *[other] { $count }টি লেআউট
    }
   *[other] { $count ->
        [one] { $count }টি উইজেট
       *[other] { $count }টি উইজেট
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count }টি স্লাইড
   *[other] { $count }টি স্লাইড
} · { $status }
ui-confirm-quit-pack = পরিবর্তিত প্যাক সংরক্ষণ না করেই বেরিয়ে যাবেন?
ui-confirm-quit-theme = পরিবর্তিত থিম সংরক্ষণ না করেই বেরিয়ে যাবেন?
ui-confirm-quit-read-only-theme = { $theme } এখানে সংরক্ষণ করা যাবে না। এই ট্যাবটি বন্ধ করে এর পরিবর্তনগুলো হারাবেন?
ui-confirm-update-opener = “{ $deck }”-কে প্যাক “{ $pack }”-এর এই সংস্করণে হালনাগাদ করবেন?
ui-close-writes-pending = এই ট্যাবটি এখনও সংরক্ষণ করছে: একটু পরে আবার বন্ধ করুন।
ui-confirm-close-writes-pending = এই ট্যাবটি এখনও সংরক্ষণ করছে। তবুও বন্ধ করে যা সংরক্ষিত হয়নি তা হারাবেন?
ui-confirm-close-writes-pending-linked = এই ট্যাবটি এখনও সংরক্ষণ করছে, আর এই প্যাক ট্যাবগুলো সংরক্ষণ ছাড়াই এর সঙ্গে বন্ধ হবে: { $tabs }। তবুও এগুলো বন্ধ করে যা সংরক্ষিত হয়নি তা হারাবেন?
ui-confirm-quit-read-only-pack = { $pack } এখানে সংরক্ষণ করা যাবে না। এই ট্যাবটি বন্ধ করে এর পরিবর্তনগুলো হারাবেন? সেগুলো রাখতে, এজেন্টকে এটি নতুন প্যাক হিসেবে সংরক্ষণ করতে বলুন।

## বিজ্ঞপ্তি

ui-toast-copied = অনুলিপি সংরক্ষিত
ui-toast-field-gone = আপনি যে ঘরটি সম্পাদনা করছিলেন সেটি আর নেই: আপনার লেখা হারিয়ে গেছে।
ui-toast-edit-replaced = আপনি সম্পাদনা করার সময় এই লেখাটি বদলে গেছে: আপনার সংস্করণ সেটির জায়গা নিয়েছে। পূর্বাবস্থায় ফেরান অন্যটি ফিরিয়ে আনে।

## টুলবার

ui-toolbar-undo = পূর্বাবস্থায় ফেরান
ui-toolbar-undo-label = পূর্বাবস্থায় ফেরান: { $label }
ui-toolbar-redo = আবার করুন
ui-toolbar-redo-label = আবার করুন: { $label }
ui-divider-pile = স্লাইড স্তূপের প্রস্থ
ui-divider-notes = এজেন্ট থ্রেডের প্রস্থ
ui-divider-code = কোডের আকার
ui-toolbar-theme = থিম
ui-toolbar-finish = সম্পন্ন
ui-pack-deck-only = শুধু এই উপস্থাপনায় সংরক্ষিত
ui-toolbar-open = খুলুন
ui-menu = মেনু
ui-toolbar-add-label = স্লাইডে যোগ করুন
ui-toolbar-packs = উপস্থাপনার প্যাক
ui-toolbar-packs-update = উপস্থাপনার প্যাক — আপডেট উপলব্ধ
ui-toolbar-thumbs-show = থাম্বনেইল দেখান
ui-toolbar-thumbs-hide = থাম্বনেইল লুকান
ui-toolbar-remove-slide = স্লাইড মুছুন
ui-toolbar-remove-hint = স্লাইড { $n } · ⌘Z দিয়ে পূর্বাবস্থায় ফেরান

## পূর্বাবস্থার লেবেল

ui-undo-add-widget = “{ $name }” যোগ করুন
ui-undo-new-slide = নতুন স্লাইড: { $name }
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
ui-undo-rename-pack = প্যাকের নাম বদলান
ui-undo-rename-deck = প্রেজেন্টেশনের নাম বদলান

## স্লাইডের তালিকা

ui-pile-slides = স্লাইড
ui-pile-layouts = লেআউট
ui-pile-add-slide = নতুন স্লাইড
ui-pile-drag-hint = সরাতে টেনে আনুন · ⌥↑ ⌥↓
ui-slide-alt = স্লাইড { $n }

## মঞ্চ

ui-stage-rendering = রেন্ডার হচ্ছে…
ui-stage-widget-pack = স্তূপ থেকে একটি উইজেট খুলুন, অথবা নতুন একটি তৈরি করুন।
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
    [chart] চার্ট
    [table] টেবিল
    [stack] স্ট্যাক
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
ui-draft = খসড়া
ui-draft-send = এজেন্টকে পাঠান
ui-draft-send-title = এজেন্টকে পাঠান (⌘↩)
ui-draft-discard = বাতিল
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = পেস্ট করা ছবি
ui-draft-remove-image = এই ছবিটি সরান
ui-draft-send-needs-text = বলুন এটি দিয়ে কী করতে হবে
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
ui-dictation-downloading = { $name } মডেল ডাউনলোড হচ্ছে
ui-dictation-download-progress = { $percent }% · { $total }-এর মধ্যে { $received } · শুধু একবার
ui-dictation-listening = শোনা হচ্ছে · { $time }
ui-dictation-transcribing = শেষ বাক্যটি লেখা হচ্ছে…
ui-dictation-stop-in = { $duration } পরে থামবে
ui-dictation-speak = বলুন, টেক্সট এখানে দেখা যাবে…
ui-dictation-auto-stop = { $duration } নীরবতার পরে থামে
ui-dictation-to-download = { $size } ডাউনলোড করতে হবে
ui-dictation-settings = সেটিংস
ui-dictation-language-auto = স্বয়ংক্রিয় শনাক্তকরণ
ui-dictation-model-unavailable = ভাষা সমর্থিত নয়
ui-dictation-language-auto-resolved = স্বয়ংক্রিয় ({ $language })
ui-dictation-to-download-language = ভাষা ডাউনলোড করতে হবে
ui-dictation-api-key-missing = { $var } সেট করা নেই
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } সে.
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## টেমপ্লেট নির্বাচক

ui-picker-new-slide = নতুন স্লাইড
ui-picker-set-mine = { $name } · আমার
ui-picker-set-received = { $name } · প্রাপ্ত
ui-picker-keys = ব্রাউজ করতে ← → · যোগ করতে Enter · বন্ধ করতে Esc
ui-picker-close = বন্ধ করুন
ui-picker-list = লেআউট
ui-widget-picker-filter = নাম, লেবেল বা বিবরণ দিয়ে ফিল্টার করুন
ui-widget-picker-empty = “{ $query }”-এর সঙ্গে কোনো উইজেট মেলে না।
ui-widget-picker-list = উইজেট
ui-widget-picker-keys = দেখতে তীরচিহ্ন · যোগ করতে Enter · বন্ধ করতে Esc

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
ui-show-as = এভাবে দেখান
ui-show-as-table = টেবিল হিসেবে দেখান
ui-show-as-chart = চার্ট হিসেবে দেখান

## ফাইল

ui-file-open-prompt = যে ফাইল খুলবেন তার পাথ (.oku-slides.pdf, .json বা .pptx)
ui-file-filter-presentations = উপস্থাপনা
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = এই হিসেবে সংরক্ষণ করুন (পূর্ণ পাথ)

## ট্যাব

ui-tabs = খোলা উপস্থাপনা
ui-tab-new = নতুন ট্যাব
ui-tab-close = ট্যাব বন্ধ করুন
ui-tab-unsaved = অসংরক্ষিত পরিবর্তন
ui-tab-pack = প্যাক
ui-tab-theme = থিম
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
ui-help-close = সহায়িকা বন্ধ করুন
ui-help-search = সহায়িকায় খুঁজুন
ui-help-search-placeholder = খুঁজুন…
ui-help-search-empty = কোনো ফল নেই
ui-help-search-count = { $count }টি ফল
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
ui-pack-tabs = প্যাকের ট্যাব
ui-pack-layouts-tab = লেআউট
ui-theme-name = থিমের নাম
ui-theme-unsaved = থিমের পরিবর্তন সংরক্ষিত হয়নি
ui-widgets-close-tab = { $name } বন্ধ করুন
ui-widgets-removed-by-agent = এজেন্ট উইজেট { $name } মুছে ফেলেছে।

## আমদানি করুন

ui-import-title = PowerPoint উপস্থাপনা আমদানি করুন
ui-import-mode = আমদানির ধরন
ui-import-hifi = হুবহু অনুলিপি
ui-import-hifi-note = এই উপস্থাপনার জন্য তৈরি থিম ও লেআউট দিয়ে মূলের চেহারা বজায় রাখে।
ui-import-retrofit = আমার লেআউট ব্যবহার করুন
ui-import-retrofit-note = লাইব্রেরির লেআউট প্যাক ও একটি থিম দিয়ে স্লাইডগুলো নতুন করে গড়ে।
ui-import-layout-packs = লেআউট প্যাক
ui-import-theme-default = ডিফল্ট থিম
ui-import-no-packs = লাইব্রেরিতে কোনো লেআউট প্যাক নেই।
ui-import-tune = এজেন্টের সাহায্যে সূক্ষ্মভাবে ঠিক করুন
ui-import-tune-note = এজেন্ট প্রতিটি স্লাইড মূলের সঙ্গে মিলিয়ে পার্থক্যগুলো ঠিক করে।
ui-import-tune-note-retrofit = এজেন্ট প্রতিটি স্লাইড মূলের সঙ্গে মিলিয়ে দেখে এবং আপনার লেআউট যে আকারগুলো দেখাতে পারে না তার জন্য উইজেট বানায়।
ui-import-tune-unavailable = ব্যবহারযোগ্য কোনো এজেন্ট নেই: { $reason }
ui-import-submit = আমদানি করুন
ui-import-busy = আমদানি হচ্ছে…
ui-update-available = Oku Slides { $version } এখন উপলব্ধ।
ui-update-install = ইনস্টল করে পুনরায় চালু করুন
ui-update-see-release = রিলিজ দেখুন
ui-update-later = পরে
ui-update-installing = আপডেট ডাউনলোড হচ্ছে… { $percent }%
ui-update-downloading = আপডেট ডাউনলোড হচ্ছে…
ui-update-up-to-date = Oku Slides সর্বশেষ সংস্করণে আছে।
ui-update-check = আপডেট খুঁজুন…
ui-about = Oku Slides সম্পর্কে
ui-about-version = সংস্করণ { $version }
ui-about-changelog = নতুন কী আছে
ui-about-breaking = অসামঞ্জস্যপূর্ণ
ui-about-close = বন্ধ করুন

ui-shortcut-theme = থিম, theme, themes

# এজেন্ট: সারফেস, নিশ্চিতকরণ, থ্রেড।

agent-surface-cancel = বাতিল
agent-surface-confirm = নিশ্চিত করুন
agent-surface-confirm-danger = হ্যাঁ, করুন
agent-surface-rewrite = পুনর্লিখন
agent-surface-pick = { $n } বেছে নিন
agent-surface-keep = মূলটি রাখুন
agent-surface-edit = প্রয়োগের আগে সম্পাদনা করুন
agent-surface-close = বন্ধ করুন
agent-surface-themes = থিম
agent-surface-variants = ভ্যারিয়েন্ট
agent-surface-permission-always = চিরদিনের জন্য
agent-surface-permission-title = এজেন্ট একটি অনুমতি চাইছে
agent-undo-turn = এজেন্ট: { $text }

agent-tools-unavailable = এজেন্টের টুলগুলি চালু করা যায়নি।

agent-undo-apply = এজেন্ট পরিবর্তন
agent-undo-variant = ভ্যারিয়েন্ট: { $label }
agent-undo-param = সেটিং: { $label }
agent-undo-rewrite = পুনর্লিখন

agent-confirm-save = উপস্থাপনাটি { $path } হিসেবে সংরক্ষণ করবেন?
agent-confirm-save-over = { $path } প্রতিস্থাপন করবেন? বিদ্যমান ফাইলটি ওভাররাইট হয়ে যাবে।
agent-confirm-open = { $path } খুলবেন?
agent-open-elsewhere = এই ফাইলটি ইতিমধ্যে অন্য একটি ট্যাবে খোলা আছে।
agent-confirm-new = নতুন উপস্থাপনা শুরু করবেন?
agent-confirm-new-dirty = নতুন উপস্থাপনা শুরু করবেন? সংরক্ষণ না করা পরিবর্তনগুলো হারিয়ে যাবে।
agent-confirm-copy-pack = প্যাক “{ $name }”-এর একটি কপি সংরক্ষণ করবেন? প্যাকটি নিজে অপরিবর্তিত থাকবে।
agent-confirm-copy-edited-theme = থিম “{ $name }”-এর একটি কপি সংরক্ষণ করবেন? থিমটি নিজে অপরিবর্তিত থাকবে।
agent-confirm-delete-pack = প্যাক “{ $name }” মুছে ফেলবেন? এটি পূর্বাবস্থায় ফেরানো যাবে না।
agent-confirm-edit-theme = “{ $name }” থিমের একটি নতুন সংস্করণ সংরক্ষণ করবেন?
agent-confirm-copy-theme = বিল্ট-ইন থিম “{ $name }”-এর একটি রূপভেদ তৈরি করবেন? বিল্ট-ইন থিম অপরিবর্তিত থাকবে।
agent-confirm-setting = { $key } পরিবর্তন করবেন: { $old } → { $new }?
agent-confirm-move = উপস্থাপনাটি { $path }-এ সরাবেন? পুরোনো ফাইলটি মুছে ফেলা হবে।
agent-confirm-merge = { $path } এই উপস্থাপনার সঙ্গে মিলিয়ে দেবেন? এটি পূর্বাবস্থায় ফেরানো যাবে না।
agent-confirm-save-copy = উপস্থাপনার একটি কপি { $path } হিসেবে সংরক্ষণ করবেন?
agent-confirm-dictation-api = আপনি যা বলে লেখাচ্ছেন তা { $host } ({ $url }) এ, মডেল { $model } দিয়ে, { $var } এর গোপন কী সহ পাঠাবেন?
agent-confirm-dictation-download = { $name } ব্যবহার করে এখনই ডাউনলোড করবেন ({ $size })?
agent-confirm-dictation-language = Apple ডিক্টেশন ব্যবহার করে এখনই এর ভাষা ডাউনলোড করবেন?
agent-confirm-agent = কোনো উত্তর চলমান না থাকলে অ্যাপের এজেন্ট { $name } ({ $command }) এ বদলাবেন?
agent-confirm-agent-builtin = কোনো উত্তর চলমান না থাকলে অ্যাপের এজেন্ট { $name } এ বদলাবেন?
agent-confirm-agent-model = লোকাল এজেন্টের মডেল ডাউনলোড করবেন ({ $size })?

agent-not-found = এজেন্ট পাওয়া যায়নি: { $command }। সেটিং [agent.presets]।
agent-spawn-failed = এজেন্ট চালু করা যায়নি: { $command }। সেটিং [agent.presets]।
agent-no-http = এই এজেন্ট Oku Slides-এর টুলের সাথে সংযোগ করতে পারে না।
agent-local-no-program = স্থানীয় এজেন্ট শুধু Oku Slides অ্যাপের ভেতরেই চলতে পারে।
agent-local-needs-download = লোকাল এজেন্টকে আগে তার মডেল ({ $gigabytes } জিবি) ডাউনলোড করতে হবে।
agent-local-intel-mac = লোকাল এজেন্টের জন্য Apple সিলিকন সহ একটি Mac দরকার।
agent-local-unsupported-os = লোকাল এজেন্ট এখনও এই সিস্টেমে উপলব্ধ নয়।
agent-model-download = মডেল ডাউনলোড করুন
agent-model-downloading = মডেল ডাউনলোড হচ্ছে… { $percent }%
agent-model-cancel = বাতিল করুন
agent-model-download-failed = মডেলটি ডাউনলোড করা যায়নি।
agent-model-download-integrity = ডাউনলোড করা মডেলটি ক্ষতিগ্রস্ত ছিল এবং মুছে ফেলা হয়েছে।
agent-model-download-incomplete = অসম্পূর্ণ ডাউনলোড: { $expected } বাইটের মধ্যে { $got } বাইট।
agent-model-download-cancelled = ডাউনলোড বাতিল করা হয়েছে।
local-agent-no-route = স্থানীয় এজেন্ট বুঝতে পারেনি কী করতে হবে। অন্যভাবে বলে দেখুন।
local-agent-tool-limit = স্থানীয় এজেন্ট { $n } ধাপের পরে কাজ শেষ না করেই থেমে গেছে।
local-agent-invalid-call = স্থানীয় এজেন্ট টুলগুলো ঠিকভাবে ব্যবহার করতে পারেনি।
local-agent-engine-failed = স্থানীয় এজেন্টের মডেল ব্যর্থ হয়েছে।
local-agent-retry = আবার চেষ্টা করুন, অথবা এজেন্ট প্যানেলে অন্য একটি এজেন্ট বেছে নিন।
agent-stopped = এজেন্ট বন্ধ হয়ে গেছে।
agent-failed = এজেন্ট অনুরোধটি প্রত্যাখ্যান করেছে: { $message }
agent-tool-failed = টুলটি অপ্রত্যাশিতভাবে ব্যর্থ হয়েছে।

agent-turn-tools = { $tools ->
   [one] { $tools }টি টুল
  *[other] { $tools }টি টুল
}
agent-turn-notes = { $notes ->
   [one] { $notes }টি নোট
  *[other] { $notes }টি নোট
}
agent-turn-secs = { $secs } সে.
agent-turn-stopped = থামানো হয়েছে।
agent-turn-interrupted = বাধাপ্রাপ্ত হয়েছে।
agent-memory-lost = এজেন্ট আর এই কথোপকথন মনে রাখে না: সে আবার শুরু থেকে শুরু করছে।
agent-turn-max-tokens = এজেন্ট দৈর্ঘ্যের সীমায় পৌঁছেছে।
agent-turn-max-turns = এজেন্ট ধাপের সীমায় পৌঁছেছে।
agent-turn-refusal = এজেন্ট প্রত্যাখ্যান করেছে।
agent-turn-ended = পালা শেষ ({ $stop })।
agent-choice-cancelled = প্রশ্ন বাতিল করা হয়েছে
agent-choice-answered = ✓ { $answer }
agent-report-why = কেন

agent-error-file = ফাইলটি পড়া বা লেখা যায়নি।
agent-error-unexpected-action = অপ্রত্যাশিত কাজ: { $action }।
agent-error-unknown-surface = এই কার্ডটি আর খোলা নেই।
agent-error-expired = আগের পালার কাজগুলোর মেয়াদ শেষ হয়ে গেছে।
agent-error-incomplete = এই কার্ডের কাজটি অসম্পূর্ণ।
agent-error-no-field = এই সেটিংটি আর নেই।
agent-error-no-variant = এই রূপটি আর নেই।
agent-error-arguments = টুলের আর্গুমেন্টগুলো অবৈধ ({ $detail }): এর স্কিমা অনুযায়ী সেগুলো যাচাই করুন।
agent-error-unknown-tool = অজানা টুল: { $name }।
agent-error-pack-id = এটি কোনো প্যাকের id নয়: প্যাক ও তাদের id-এর তালিকা পেতে packs ডাকুন।
agent-error-asset-unknown = এই ছবিটি উপস্থাপনায় নেই, আবার পেস্ট করা ছবিগুলোর মধ্যেও নেই: পেস্ট করা কোনো ছবির আইডি ব্যবহার করুন।
agent-error-denied = ব্যবহারকারী প্রত্যাখ্যান করেছেন।
agent-error-no-turn = কোনো পালা চলছে না: { $tool } শুধু একটি পালার সময় দেখানো হয়।
agent-error-tab-no-turn = ব্যবহারকারীকে উত্তর দেওয়ার সময়ই কেবল প্যাক বা থিম ট্যাব খুলুন বা দেখান।
agent-error-tab-limit = এই পালায় আপনি ইতিমধ্যে একটি প্যাক বা থিম ট্যাব খুলেছেন বা দেখিয়েছেন; সেখানেই শেষ করুন, অথবা আরেকটির আগে ব্যবহারকারীকে জিজ্ঞাসা করুন।
agent-error-continue-in-other-tab = এই অনুরোধ আপনার খোলা বা দেখানো ট্যাবে চলবে: ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন; এই ট্যাব আপাতত কোনো উইজেট বদলাবে না।
agent-error-pack-open-in-tab = এই প্যাকটি অন্য একটি ট্যাবে সম্পাদনা করা হচ্ছে: ব্যবহারকারীকে আগে সেখানে শেষ করতে বলুন।
agent-pack-opened-in-tab = { $shown ->
   [yes] প্যাকটি { $tab } ট্যাবে খুলেছে, এখন দেখানো হচ্ছে।
  *[no] প্যাকটি { $tab } ট্যাবে পেছনে খুলেছে: ব্যবহারকারীর উইন্ডোতে অন্য একটি ট্যাব দেখানো হচ্ছে।
} { $draft ->
   [set] অনুরোধটি ওই ট্যাবের খসড়ায় অপেক্ষা করছে: ব্যবহারকারীকে সেখানে এটি দেখে নিয়ে তারপর পাঠাতে বলুন।
   [kept] ওই ট্যাবের খসড়ায় আগে থেকেই কিছু ছিল, সেটি যেমন ছিল তেমনই রাখা হয়েছে। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
   [too_long] অনুরোধটি ওই ট্যাবের খসড়ার জন্য খুব দীর্ঘ ছিল। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
   [unavailable] অনুরোধটি ওই ট্যাবের খসড়ায় পৌঁছাতে পারেনি। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
  *[none] ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
} এই ট্যাব যেমন আছে তেমনই থাকে।
agent-theme-opened-in-tab = { $shown ->
   [yes] থিমটি { $tab } ট্যাবে খুলেছে, এখন দেখানো হচ্ছে।
  *[no] থিমটি { $tab } ট্যাবে পেছনে খুলেছে: ব্যবহারকারীর উইন্ডোতে অন্য একটি ট্যাব দেখানো হচ্ছে।
} { $draft ->
   [set] অনুরোধটি ওই ট্যাবের খসড়ায় অপেক্ষা করছে: ব্যবহারকারীকে সেখানে এটি দেখে নিয়ে তারপর পাঠাতে বলুন।
   [kept] ওই ট্যাবের খসড়ায় আগে থেকেই কিছু ছিল, সেটি যেমন ছিল তেমনই রাখা হয়েছে। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
   [too_long] অনুরোধটি ওই ট্যাবের খসড়ার জন্য খুব দীর্ঘ ছিল। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
   [unavailable] অনুরোধটি ওই ট্যাবের খসড়ায় পৌঁছাতে পারেনি। ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
  *[none] ব্যবহারকারীকে সেখানে চালিয়ে যেতে বলুন: ওই ট্যাবের এজেন্ট এই অনুরোধ জানে না।
} এই ট্যাব যেমন আছে তেমনই থাকে।
agent-error-turn-over = এই কলের পালা শেষ হয়ে গেছে: কিছুই দেখানো হয়নি।
agent-error-no-file = উপস্থাপনার এখনও কোনো ফাইল নেই: একটি সম্পূর্ণ পাথ দিন।
agent-error-path-relative = পাথটি সম্পূর্ণ (absolute) হতে হবে।
agent-error-path-parent = পাথে “..” থাকা চলবে না।
agent-error-setting-locked = এজেন্ট { $key } সেটিংটি পরিবর্তন করতে পারে না।
agent-error-setting = { $key } সেটিংটি পরিবর্তন করা যায়নি: এর মান যাচাই করুন।
agent-error-history-index = ইতিহাসে চলাচলের জন্য একটি সংস্করণের ক্রমিক নম্বর দরকার।
agent-error-no-pack = এই উপস্থাপনা কোনো প্যাক পিন করে না: pack দিন (packs দেখুন)।
agent-error-history-not-opened = ট্যাবটি সময়মতো ইতিহাস খোলেনি: আবার বলুন।
agent-error-present = এখানে উপস্থাপনের কিছু নেই: কোনো স্লাইড নেই, অথবা ট্যাবটি প্যাক, ইতিহাস বা শুধু-পড়ার মোডে আছে।
agent-error-locale = অজানা ভাষার কোড। একটি অন্তর্ভুক্ত ভাষার কোড অথবা system দিন।
agent-error-empty-interface = বদলানোর কিছু নেই: scheme, locale অথবা guides দিন।
agent-error-empty-dictation = বদলানোর কিছু নেই: ডিক্টেশনের অন্তত একটি সেটিং দিন।
agent-error-not-shown = এই ট্যাবটি তার উইন্ডোতে দেখানো ট্যাব নয়: ব্যবহারকারী এটি দেখালে আবার বলুন।
agent-error-themes-deck-mode = থিম নির্বাচক কেবল স্লাইড সম্পাদনার সময় খোলে।
agent-error-packs-deck-mode = উপস্থাপনার প্যাক প্যানেল কেবল স্লাইড সম্পাদনার সময় খোলে।
agent-error-usage-no-deck = এই ট্যাবের ব্যবহার কোনো উপস্থাপনার নয়: কোনো উপস্থাপনা থেকে খোলা প্যাক বা থিম ট্যাব (ব্যবহারকারী বা এজেন্ট যে-ই খুলুক), অথবা উপস্থাপনার নিজের প্যাক, কেবল সেটিই সেই উপস্থাপনায় গণনা হয়।
agent-error-rewrite-text = পুনর্লিখন কোনো লেখা ছাড়াই ফিরে এসেছে।
agent-error-paragraphs = পুনর্লিখনের অনুচ্ছেদগুলো অবৈধ।
agent-error-draft-too-long = খসড়াটি অনেক দীর্ঘ।
agent-error-draft-origin = খসড়া প্রত্যাখ্যাত: অজানা ভিউ।
agent-error-draft-no-agent = এই ট্যাবের এজেন্ট এখনও চালু হয়নি: খসড়া প্রত্যাখ্যাত।
agent-error-too-many-images = খসড়ায় ইতিমধ্যে সব ছবি আছে।
agent-error-image-not-base64 = পেস্ট করা ছবিটি পুরোপুরি পৌঁছায়নি।
agent-error-image-gone = পেস্ট করা একটি ছবি আর পাওয়া যাচ্ছে না: আবার পেস্ট করুন।
agent-error-unknown-agent = এই এজেন্ট আর কনফিগারেশনে নেই।
agent-error-agent-locked = এজেন্টের পছন্দ পরিবেশ দ্বারা নির্ধারিত।
agent-error-agent-not-saved = এজেন্টের পছন্দ সংরক্ষণ করা যায়নি।
agent-error-unknown-model = এই এজেন্ট ওই মডেলটি দেয় না।
agent-error-model-not-saved = মডেলের পছন্দ সংরক্ষণ করা যায়নি।
voice-error-no-dictation = এখানে ডিকটেশন উপলব্ধ নয়
voice-error-busy = ডিকটেশন চলছে: থামলে আবার চেষ্টা করুন
voice-error-unknown-model = অজানা ডিকটেশন মডেল
voice-error-unavailable = এই মডেল বেছে নেওয়া ভাষা সমর্থন করে না
voice-error-installed = এই মডেল আগেই ইনস্টল করা আছে
voice-error-not-downloadable = এই মডেল ডাউনলোড হয় না
voice-error-nothing-to-cancel = কোনো ডাউনলোড চলছে না
voice-error-bad-silence = নীরবতা 0.5 থেকে 10 সেকেন্ড হতে হবে
voice-error-bad-language = অজানা ডিকটেশন ভাষা
voice-error-bad-url = API ঠিকানা https:// দিয়ে শুরু হতে হবে (http:// শুধু এই কম্পিউটারের জন্য), ব্যবহারকারীর নাম, প্যারামিটার বা বিশেষ অক্ষর ছাড়া
voice-error-bad-key-env = কী-এর ভেরিয়েবল OPENAI_API_KEY-এর মতো একটি নাম হতে হবে: বড় হাতের অক্ষর, সংখ্যা ও _
voice-error-bad-api-model = API মডেলের নামে কেবল অক্ষর, সংখ্যা এবং . _ : / - থাকতে পারে; Azure ডিপ্লয়মেন্টের নামে কেবল অক্ষর, সংখ্যা এবং . _ -
voice-error-bad-api-version = Azure API সংস্করণে কেবল অক্ষর, সংখ্যা, . এবং - থাকতে পারে, সর্বোচ্চ 32টি অক্ষর, যেমন 2025-03-01-preview
voice-error-bad-api-headers = হেডার Name: value; Name: $VARIABLE আকারে লেখা হয়, সর্বোচ্চ 8টি। Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection এবং পুনরাবৃত্তি প্রত্যাখ্যাত হয়
voice-error-api-reserved = API-এর ঠিকানা, কী-এর ভেরিয়েবল, Azure সংস্করণ ও হেডার কেবল ব্যবহারকারী ঠিক করেন: show_dictation দিয়ে বাক্সটি খুলুন এবং তাঁকে বলুন
voice-error-changed = প্রশ্নের পর থেকে সেটিংস বদলেছে: আবার জিজ্ঞেস করুন
voice-error-nothing-to-download = এই এজেন্টের জন্য ডাউনলোড করার কিছু নেই
agent-tune-prompt = আমদানি করা উপস্থাপনাটি সূক্ষ্মভাবে ঠিক করো
agent-settings-unknown-selected = সেটিং agent.selected = { $selected } [agent.presets]-এর কোনো এজেন্টের নাম নয় (পরিচিত: { $known })
agent-settings-preset-launch = এজেন্ট [agent.presets.{ $id }]-এর command বা builtin-এর ঠিক একটি দরকার

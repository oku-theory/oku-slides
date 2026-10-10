# এজেন্ট: সারফেস, নিশ্চিতকরণ, থ্রেড।

agent-surface-cancel = বাতিল
agent-surface-confirm = নিশ্চিত করুন
agent-surface-confirm-danger = হ্যাঁ, করুন
agent-surface-rewrite = পুনর্লিখন
agent-surface-pick = { $n } বেছে নিন
agent-surface-keep = মূলটি রাখুন
agent-surface-edit = প্রয়োগের আগে সম্পাদনা করুন
agent-surface-close = বন্ধ করুন
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
agent-confirm-save-templates = টেমপ্লেট সেট “{ $name }” সংরক্ষণ করবেন?
agent-confirm-delete-set = টেমপ্লেট সেট “{ $name }” মুছে ফেলবেন? এটি পূর্বাবস্থায় ফেরানো যাবে না।
agent-confirm-edit-theme = “{ $name }” থিমের একটি নতুন সংস্করণ সংরক্ষণ করবেন?
agent-confirm-copy-theme = বিল্ট-ইন থিম “{ $name }”-এর একটি সম্পাদিত কপি সংরক্ষণ করবেন?
agent-confirm-setting = { $key } পরিবর্তন করবেন: { $old } → { $new }?

agent-not-found = এজেন্ট পাওয়া যায়নি: { $command }। সেটিং [agent.presets]।
agent-no-http = এই এজেন্ট Oku Slides-এর টুলের সাথে সংযোগ করতে পারে না।
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
agent-error-denied = ব্যবহারকারী প্রত্যাখ্যান করেছেন।
agent-error-no-turn = কোনো পালা চলছে না: { $tool } শুধু একটি পালার সময় দেখানো হয়।
agent-error-turn-over = এই কলের পালা শেষ হয়ে গেছে: কিছুই দেখানো হয়নি।
agent-error-anchor-scope = উইজেট কথোপকথনের প্রশ্ন কোনো স্লাইডে নোঙর করা যায় না: নোঙর ছাড়াই জিজ্ঞাসা করুন।
agent-error-no-file = উপস্থাপনার এখনও কোনো ফাইল নেই: একটি সম্পূর্ণ পাথ দিন।
agent-error-path-relative = পাথটি সম্পূর্ণ (absolute) হতে হবে।
agent-error-path-parent = পাথে “..” থাকা চলবে না।
agent-error-setting-locked = এজেন্ট { $key } সেটিংটি পরিবর্তন করতে পারে না।
agent-error-setting = { $key } সেটিংটি পরিবর্তন করা যায়নি: এর মান যাচাই করুন।
agent-error-rewrite-text = পুনর্লিখন কোনো লেখা ছাড়াই ফিরে এসেছে।
agent-error-paragraphs = পুনর্লিখনের অনুচ্ছেদগুলো অবৈধ।
agent-error-draft-too-long = খসড়াটি অনেক দীর্ঘ।
agent-error-draft-origin = খসড়া প্রত্যাখ্যাত: অজানা ভিউ।
agent-error-draft-no-agent = এই ট্যাবের এজেন্ট এখনও চালু হয়নি: খসড়া প্রত্যাখ্যাত।
agent-error-unknown-agent = এই এজেন্ট আর কনফিগারেশনে নেই।
agent-error-agent-locked = এজেন্টের পছন্দ পরিবেশ দ্বারা নির্ধারিত।
agent-error-agent-not-saved = এজেন্টের পছন্দ সংরক্ষণ করা যায়নি।
agent-picker-title = এজেন্ট
agent-picker-busy = এজেন্ট বদলালে চলমান পালা বন্ধ হয়ে যাবে।
agent-picker-locked = পরিবেশ এই পছন্দ নির্ধারণ করেছে (OKU_SLIDES_AGENT__SELECTED)।

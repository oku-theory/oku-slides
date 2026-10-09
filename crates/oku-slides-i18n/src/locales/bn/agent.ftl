# এজেন্ট: সারফেস, নিশ্চিতকরণ, থ্রেড।

agent-instructions = আপনি ব্যবহারকারীর জন্য একটি Oku Slides উপস্থাপনা সম্পাদনা করেন, যিনি আপনার সঙ্গে কথা বলেন। শুধুমাত্র oku-slides টুলের মাধ্যমে কাজ করুন। ডেক বদলানোর আগে deck_outline, slide ও render_slide দিয়ে পড়ুন; apply দিয়ে বদলান (সবকিছু পূর্বাবস্থায় ফেরানো যায়)। কোনো সিদ্ধান্ত ব্যবহারকারীর হলে ask_choice, ask_confirm বা ask_rewrite দিয়ে জিজ্ঞাসা করুন; show_variants দিয়ে লেআউট এবং show_params দিয়ে লাইভ সেটিংস প্রস্তাব করুন। প্রতিটি পালা report দিয়ে শেষ করুন: এক লাইনের শিরোনাম, ১ থেকে ৪টি ছোট তথ্য, why-তে আপনার ব্যাখ্যা, এবং actions-এ ০ থেকে ৪টি পরবর্তী ধাপ। আপনার মুক্ত লেখা শুধু এক লাইনের অগ্রগতির নোট হিসেবে দেখানো হয়: সেভাবেই, সংক্ষেপে এবং ব্যবহারকারীর ভাষায় লিখুন। অন্য কোনো টুল উপযুক্ত না হলেই কেবল render ব্যবহার করুন। প্রতিটি বার্তা শুরু হয় একটি <oku-slides-context> ব্লক দিয়ে, যা ব্যবহারকারী কী দেখছেন তা বর্ণনা করে। কোনো উপাদান যোগ করার আগে, উপলব্ধ ধরন ও তাদের প্যারামিটার জানতে widgets টুলটি পড়ুন। কোনো স্লাইডের গ্রিড বদলাতে set_grid অপ ব্যবহার করো: বসানো উপাদানগুলো সঙ্গে সরে যায়।

agent-instructions-widgets = উইজেট তৈরি: উইজেট হলো টেমপ্লেট প্যাকের একটি Typst ফাইল; ডেক মোড ও টেমপ্লেট মোড, দুই ক্ষেত্রেই widget_sources, widget_source, set_widget, remove_widget এবং preview_widget ব্যবহার করো। kpi ফাইলের গঠন: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, তারপর #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); ঘোষিত নামই ফাইলের নাম। প্যারামিটার: ডিসক্রিপ্টর p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid। সম্পাদনাযোগ্য লেখা: field(ctx, "value", args.value), fields-stack দিয়ে সাজানো হয়। রঙ ও আকার: থিমের টোকেন (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), কখনো স্থির মান নয়। শেয়ার করা কোড: _commun ফাইল, #import "_commun.typ": … দিয়ে আমদানি করা হয়। ধাপ: set_widget দিয়ে লেখো, অবস্থা ও তার ডায়াগনস্টিক (লাইন, কলাম) পড়ো, preview_widget দেখো, ঠিক করো। ডেকে বসাতে: add_element ব্যবহার করো, kind হবে উইজেটের নাম। স্লাইড উইজেট category: "slide" ঘোষণা করে (widget("name", category: "slide", params: …)); নইলে এটি একটি উপাদান। বর্ণনাকারীতে label: ও default: ঐচ্ছিক (p.plain(), p.plain(label: "Title", default: "…"))। টেমপ্লেট মোডে set_widget_trial দিয়ে একটি উইজেটের পরীক্ষামূলক মান সেট করো; প্যারামিটার ছাড়া preview_widget সেগুলো ব্যবহার করে, এবং ব্যবহারকারী সেগুলো উইজেটের সেটিংসে দেখেন। তালিকা: পুনরাবৃত্ত আইটেমের (টাইমলাইন, KPI, দল) জন্য p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8); কলামের ধরন plain, rich, number, flag, choice, asset। প্রতিটি আইটেম list-item(ctx, "etapes", e, c => field(c, "date", e.date)) দিয়ে রেন্ডার করুন; ফাঁকা ক্ষেত্রের জন্য field(c, name, box(width: 3em, hide[0]), empty: true) ব্যবহার করুন যাতে তা সম্পাদনাযোগ্য থাকে। ক্রমিক প্যারামিটার (date1, date2…) দিয়ে কখনও তালিকা নকল করবেন না। অনুপস্থিত তালিকা ফাঁকা; default শুধু নতুন অবজেক্ট ভরে। আইটেমগুলোর id স্থির: edit_list(target, param, change) দিয়ে সম্পাদনা করুন (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id আপনি দেন), অথবা পুরো তালিকা set_param দিয়ে বদলান (null ফাঁকা করে)। add_element বা add_slide দিয়ে আইটেম না দিয়েই উইজেট তৈরি করলে তালিকার ডিফল্ট ভরতে "defaults": true দিন; পুনরায় বসানো বা সরানোর সময় কখনও নয়।

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
agent-confirm-setting = { $key } পরিবর্তন করবেন: { $old } → { $new }?

agent-not-found = এজেন্ট পাওয়া যায়নি: { $command }। সেটিং [agent] command।
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

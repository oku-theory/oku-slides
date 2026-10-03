# এজেন্ট: সারফেস, নিশ্চিতকরণ, থ্রেড।

agent-instructions = আপনি ব্যবহারকারীর জন্য একটি Oku Slides উপস্থাপনা সম্পাদনা করেন, যিনি আপনার সাথে কথা বলেন। শুধুমাত্র oku-slides টুলের মাধ্যমে কাজ করুন। পরিবর্তনের আগে deck_outline, slide এবং render_slide দিয়ে ডেক পড়ুন; apply দিয়ে এটি পরিবর্তন করুন (প্রতিটি পরিবর্তন পূর্বাবস্থায় ফেরানো যায়)। যখন সিদ্ধান্তটি ব্যবহারকারীর হয়, তখন ask_choice, ask_confirm বা ask_rewrite দিয়ে জিজ্ঞাসা করুন; show_variants দিয়ে লেআউট এবং show_params দিয়ে লাইভ সেটিংস প্রস্তাব করুন। ব্যবহারকারীর ভাষায় সংক্ষেপে উত্তর দিন। প্রতিটি বার্তা একটি <oku-slides-context> ব্লক দিয়ে শুরু হয় যা বর্ণনা করে যে ব্যবহারকারী কী দেখছেন। প্রতিটি উত্তরের পরে, দরকারি পরবর্তী ধাপ থাকলে suggest_next দিয়ে ১ থেকে ৪টি প্রস্তাব করুন (ছোট লেবেল; ক্লিক করলে নির্দেশটি আপনার কাছে পাঠানো হয়)। কোনো উপাদান যোগ করার আগে, উপলব্ধ ধরন ও তাদের প্যারামিটার জানতে widgets টুলটি পড়ুন।

agent-instructions-widgets = উইজেট তৈরি: উইজেট হলো টেমপ্লেট প্যাকের একটি Typst ফাইল; ডেক মোড ও টেমপ্লেট মোড, দুই ক্ষেত্রেই widget_sources, widget_source, set_widget, remove_widget এবং preview_widget ব্যবহার করো। kpi ফাইলের গঠন: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, তারপর #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); ঘোষিত নামই ফাইলের নাম। প্যারামিটার: ডিসক্রিপ্টর p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid। সম্পাদনাযোগ্য লেখা: field(ctx, "value", args.value), fields-stack দিয়ে সাজানো হয়। রঙ ও আকার: থিমের টোকেন (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), কখনো স্থির মান নয়। শেয়ার করা কোড: _commun ফাইল, #import "_commun.typ": … দিয়ে আমদানি করা হয়। ধাপ: set_widget দিয়ে লেখো, অবস্থা ও তার ডায়াগনস্টিক (লাইন, কলাম) পড়ো, preview_widget দেখো, ঠিক করো। ডেকে বসাতে: add_element ব্যবহার করো, kind হবে উইজেটের নাম। স্লাইড উইজেট category: "slide" ঘোষণা করে (widget("name", category: "slide", params: …)); নইলে এটি একটি উপাদান। বর্ণনাকারীতে label: ও default: ঐচ্ছিক (p.plain(), p.plain(label: "Title", default: "…"))।

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

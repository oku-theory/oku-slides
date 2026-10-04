# এজেন্ট: সারফেস, নিশ্চিতকরণ, থ্রেড।

agent-instructions = আপনি ব্যবহারকারীর জন্য একটি Oku Slides উপস্থাপনা সম্পাদনা করেন, যিনি আপনার সঙ্গে কথা বলেন। শুধুমাত্র oku-slides টুলের মাধ্যমে কাজ করুন। ডেক বদলানোর আগে deck_outline, slide ও render_slide দিয়ে পড়ুন; apply দিয়ে বদলান (সবকিছু পূর্বাবস্থায় ফেরানো যায়)। কোনো সিদ্ধান্ত ব্যবহারকারীর হলে ask_choice, ask_confirm বা ask_rewrite দিয়ে জিজ্ঞাসা করুন; show_variants দিয়ে লেআউট এবং show_params দিয়ে লাইভ সেটিংস প্রস্তাব করুন। প্রতিটি পালা report দিয়ে শেষ করুন: এক লাইনের শিরোনাম, ১ থেকে ৪টি ছোট তথ্য, why-তে আপনার ব্যাখ্যা, এবং actions-এ ০ থেকে ৪টি পরবর্তী ধাপ। আপনার মুক্ত লেখা শুধু এক লাইনের অগ্রগতির নোট হিসেবে দেখানো হয়: সেভাবেই, সংক্ষেপে এবং ব্যবহারকারীর ভাষায় লিখুন। অন্য কোনো টুল উপযুক্ত না হলেই কেবল render ব্যবহার করুন। প্রতিটি বার্তা শুরু হয় একটি <oku-slides-context> ব্লক দিয়ে, যা ব্যবহারকারী কী দেখছেন তা বর্ণনা করে। কোনো উপাদান যোগ করার আগে, উপলব্ধ ধরন ও তাদের প্যারামিটার জানতে widgets টুলটি পড়ুন। কোনো স্লাইডের গ্রিড বদলাতে set_grid অপ ব্যবহার করো: বসানো উপাদানগুলো সঙ্গে সরে যায়।

agent-instructions-widgets = উইজেট তৈরি: উইজেট হলো টেমপ্লেট প্যাকের একটি Typst ফাইল; ডেক মোড ও টেমপ্লেট মোড, দুই ক্ষেত্রেই widget_sources, widget_source, set_widget, remove_widget এবং preview_widget ব্যবহার করো। kpi ফাইলের গঠন: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, তারপর #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); ঘোষিত নামই ফাইলের নাম। প্যারামিটার: ডিসক্রিপ্টর p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid। সম্পাদনাযোগ্য লেখা: field(ctx, "value", args.value), fields-stack দিয়ে সাজানো হয়। রঙ ও আকার: থিমের টোকেন (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), কখনো স্থির মান নয়। শেয়ার করা কোড: _commun ফাইল, #import "_commun.typ": … দিয়ে আমদানি করা হয়। ধাপ: set_widget দিয়ে লেখো, অবস্থা ও তার ডায়াগনস্টিক (লাইন, কলাম) পড়ো, preview_widget দেখো, ঠিক করো। ডেকে বসাতে: add_element ব্যবহার করো, kind হবে উইজেটের নাম। স্লাইড উইজেট category: "slide" ঘোষণা করে (widget("name", category: "slide", params: …)); নইলে এটি একটি উপাদান। বর্ণনাকারীতে label: ও default: ঐচ্ছিক (p.plain(), p.plain(label: "Title", default: "…"))। টেমপ্লেট মোডে set_widget_trial দিয়ে একটি উইজেটের পরীক্ষামূলক মান সেট করো; প্যারামিটার ছাড়া preview_widget সেগুলো ব্যবহার করে, এবং ব্যবহারকারী সেগুলো উইজেটের সেটিংসে দেখেন।

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
   [one] { $tools } টুল
  *[other] { $tools } টুল
}
agent-turn-notes = { $notes ->
   [one] { $notes } নোট
  *[other] { $notes } নোট
}
agent-turn-secs = { $secs } সে.
agent-turn-stopped = থামানো হয়েছে।
agent-turn-interrupted = বাধাপ্রাপ্ত হয়েছে।
agent-turn-max-tokens = এজেন্ট দৈর্ঘ্যের সীমায় পৌঁছেছে।
agent-turn-max-turns = এজেন্ট ধাপের সীমায় পৌঁছেছে।
agent-turn-refusal = এজেন্ট প্রত্যাখ্যান করেছে।
agent-turn-ended = পালা শেষ ({ $stop })।
agent-choice-cancelled = প্রশ্ন বাতিল করা হয়েছে
agent-choice-answered = ✓ { $answer }
agent-report-why = কেন

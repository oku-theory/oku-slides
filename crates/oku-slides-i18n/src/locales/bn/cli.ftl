cli-reading = { $path } পড়া হচ্ছে
cli-invalid-deck = অবৈধ প্রেজেন্টেশন: { $path }
cli-deck-valid = বৈধ প্রেজেন্টেশন: { $slides }টি স্লাইড
cli-deck-issue = সমস্যা: { $issue }
cli-rendered = { $pages } পৃষ্ঠা → { $out } (কম্পাইলেশন { $compile }, SVG { $svg })
cli-asset-added = অ্যাসেট { $path } → { $id }
cli-saved = { $path }: { $pages } পৃষ্ঠা, PDF { $pdf } KB, উৎস { $sources } KB
cli-opened = { $path }: { $slides }টি স্লাইড, { $assets }টি অ্যাসেট, { $fonts }টি ফন্ট
cli-extracted = { $dir }-এ বের করা হয়েছে
cli-stripped = { $path }: উৎস ছাড়া কপি
cli-pdf-modified = Oku Slides-এর পরে অন্য একটি টুল PDF পরিবর্তন করেছে
cli-iterations = --iterations অবশ্যই 0-এর বেশি হতে হবে
cli-no-block = প্রেজেন্টেশনে সম্পাদনা করার মতো কোনো ব্লক নেই
cli-blob-missing = উৎস ব্লব পাওয়া যায়নি
cli-roundtrip-mismatch = পুনরায় পড়া প্রেজেন্টেশন সংরক্ষিতটির থেকে আলাদা
cli-bench-world = পরিবেশ তৈরি (ফন্ট)               { $time }
cli-bench-first = প্রথম কম্পাইলেশন                 { $time }  ({ $pages } পৃষ্ঠা)
cli-bench-same = পরিবর্তন ছাড়া পুনঃকম্পাইলেশন     { $time }
cli-bench-edited-text = সম্পাদিত টেক্সট { $n }
cli-bench-edit = set_text-এর পরে পুনঃকম্পাইলেশন    { $time }
cli-bench-svg = প্রতিটি স্লাইডের SVG               { $time }  ({ $each } প্রতি স্লাইড)
cli-bench-pdf = PDF এক্সপোর্ট                      { $time }  ({ $size } KB)
cli-bench-embed = উৎস ব্লব এম্বেড করা               { $time }
cli-bench-extract = ব্লব আবার পড়া                    { $time }
cli-bench-roundtrip = প্রেজেন্টেশনের রাউন্ড ট্রিপ        অভিন্ন
cli-bench-history = ইতিহাসের ধাপ                    গড়ে { $mean } ms, p50 { $p50 } ms
cli-serving = Oku Slides (dev) চলছে http://{ $host }:{ $port }-এ
cli-serving-token = { $url } খুলুন (সার্ভার বন্ধ না হওয়া পর্যন্ত টোকেনটি বৈধ)
cli-models-dir = ডিক্টেশন মডেল: { $dir }
cli-stopped = সার্ভার বন্ধ হয়েছে
cli-not-found = পাওয়া যায়নি
cli-bad-request = অবৈধ অনুরোধ: { $reason }
cli-sse-full = ইতিমধ্যে অনেক বেশি ইভেন্ট স্ট্রিম খোলা আছে
cli-forbidden = অনুরোধ প্রত্যাখ্যাত: এটি স্থানীয় ইন্টারফেস থেকে আসেনি
cli-unauthorized = অনুরোধ প্রত্যাখ্যাত: অ্যাক্সেস টোকেন নেই বা ভুল
cli-exposed = { $host }-এ শুনছে: সার্ভারটি নেটওয়ার্ক থেকে পৌঁছানো যায়; শুধু যে ক্লায়েন্টরা এই চালুর টোকেন দেখায় তারাই পরিষেবা পায়। নেটওয়ার্ক দরকার না হলে [serve] host 127.0.0.1-এ রাখুন
cli-imported = { $path }: { $slides ->
       [one] { $slides }টি স্লাইড
      *[other] { $slides }টি স্লাইড
    }, { $pages ->
       [one] { $pages } পৃষ্ঠা
      *[other] { $pages } পৃষ্ঠা
    }, { $losses ->
       [one] { $losses }টি ক্ষতি
      *[other] { $losses }টি ক্ষতি
    }, { $requests ->
       [one] { $requests }টি উইজেট অনুরোধ
      *[other] { $requests }টি উইজেট অনুরোধ
    }
cli-import-pack-required = retrofit মোডে --pack আবশ্যক: টেমপ্লেট প্যাকের আইডি, ফোল্ডার বা নাম
cli-import-write-failed = ইমপোর্ট করা ডেক { $path } লেখা যায়নি
cli-import-hifi-no-target = --pack ও --theme শুধু retrofit মোডের জন্য: উচ্চ বিশ্বস্ততার ইমপোর্ট (--mode hifi) নিজের প্যাক ও থিম তৈরি করে
cli-import-reference-hifi-only = --reference hifi মোডের অংশ: retrofit আমদানি রেফারেন্স রেন্ডারের সঙ্গে তুলনা করা হয় না
cli-import-scored = { $witness }-এর সঙ্গে দৃশ্যমান পার্থক্য: গড়ে { $mean }, { $threshold }-এর উপরে { $above ->
    [0] কোনো স্লাইড নেই
    [one] { $above }টি স্লাইড
   *[other] { $above }টি স্লাইড
}
cli-import-unscored = { $reason }

cli-about = Oku Slides প্রেজেন্টেশন রেন্ডার ও সংরক্ষণ করুন
cli-arg-fonts = অতিরিক্ত ফন্ট ফোল্ডার (TTF/OTF); ডিফল্ট: সেটিংসে paths.fonts
cli-cmd-check = একটি প্রেজেন্টেশন যাচাই করুন
cli-arg-deck = প্রেজেন্টেশন (JSON)
cli-cmd-render = প্রতিটি স্লাইড উপাদান বক্সসহ SVG-তে রেন্ডার করুন
cli-arg-input = deck.json বা .oku-slides.pdf ফাইল
cli-arg-out = আউটপুট ফোল্ডার
cli-arg-theme = প্রেজেন্টেশনের থিম প্রতিস্থাপন করুন: অন্তর্নির্মিত বা লাইব্রেরির থিম, নাম দিয়ে
cli-cmd-save = প্রেজেন্টেশন .oku-slides.pdf হিসেবে সংরক্ষণ করুন (PDF + উৎস ব্লব)
cli-arg-output = যে ফাইলে লেখা হবে
cli-arg-assets = অ্যাসেট ফোল্ডার: প্রতিটি ফাইল একটি sha256:… অ্যাসেট হয়
cli-arg-embed-fonts = --fonts-এর ফন্টগুলোও উৎসে এম্বেড করুন
cli-cmd-open = একটি .oku-slides.pdf আবার খুলুন: প্রেজেন্টেশন ও এর অ্যাসেট বের করুন
cli-arg-extract = যে ফোল্ডারে deck.json, assets/ ও fonts/ লেখা হবে
cli-cmd-strip = উৎস ব্লব ছাড়া একটি .oku-slides.pdf কপি করুন
cli-cmd-bench = কম্পাইলেশন ও এক্সপোর্টের সময় মাপুন
cli-arg-iterations = প্রতি ধাপে পরিমাপের সংখ্যা
cli-cmd-schema = অপারেশনগুলোর JSON স্কিমা প্রিন্ট করুন
cli-cmd-serve = ডেভেলপমেন্ট সার্ভার: ব্রাউজারে ইন্টারফেস, Tauri ছাড়া
cli-arg-serve-input = যে প্রেজেন্টেশন খোলা হবে (.oku-slides.pdf বা .json)
cli-arg-port = পোর্ট; ডিফল্ট: সেটিংসে serve.port
cli-arg-ui = বিল্ড করা ইন্টারফেসের ফোল্ডার (ui/dist)
cli-cmd-import = একটি PowerPoint প্রেজেন্টেশন (.pptx) .oku-slides.pdf হিসেবে ইমপোর্ট করুন
cli-arg-pptx = PowerPoint প্রেজেন্টেশন (.pptx)
cli-arg-mode = retrofit: বিদ্যমান টেমপ্লেট প্যাক ও থিমে বিষয়বস্তু; hifi: হুবহু কপি, এর জন্য তৈরি প্যাক ও থিমে
cli-arg-pack = লক্ষ্য টেমপ্লেট প্যাক: আইডি, প্যাক ফোল্ডার (pack.json, widgets/) বা নাম
cli-arg-import-theme = লক্ষ্য থিম: অন্তর্নির্মিত বা লাইব্রেরির থিম, আইডি বা নাম দিয়ে; ডিফল্ট: ডিফল্ট থিম
cli-arg-report = ইমপোর্ট রিপোর্টও (JSON) লিখুন: ক্ষতি, বিকল্প, কভারেজ
cli-arg-no-system-fonts = hifi মোডে, উপস্থাপনার ফন্ট সিস্টেম ফন্টে খুঁজবেন না
cli-arg-reference = hifi মোডে, প্রতিটি স্লাইডকে উপস্থাপনার রেফারেন্স রেন্ডারের সঙ্গে তুলনা করুন: auto (ডিফল্ট: [import] witness, না থাকলে office2pdf), office2pdf, soffice (LibreOffice) বা না (none)

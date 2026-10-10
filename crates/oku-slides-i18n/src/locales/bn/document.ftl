pdf-unreadable = PDF পড়া যাচ্ছে না
pdf-unsupported-version = এই PDF Oku Slides-এর নতুন সংস্করণে লেখা হয়েছে
pdf-corrupted = এই PDF-এর উৎসগুলো ক্ষতিগ্রস্ত
pdf-write = PDF লেখা যায়নি

render-compile = প্রেজেন্টেশন রেন্ডার করা যায়নি
render-pdf = PDF এক্সপোর্ট ব্যর্থ হয়েছে
render-no-page = এই পৃষ্ঠাটি নেই
render-image = স্লাইডের ছবি তৈরি করতে পারা যায়নি

render-setup = রেন্ডারার চালু করা যায়নি
render-missing-image = ছবি অনুপস্থিত
diagnostic-error = ত্রুটি
diagnostic-warning = সতর্কতা
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = ইঙ্গিত: { $hint }
render-chart-failed = চার্ট আঁকা যায়নি

registry-bad-pack-id = অবৈধ টেমপ্লেট প্যাক শনাক্তকারী: { $id }
registry-compile = টেমপ্লেট প্যাকের উইজেটগুলি কম্পাইল করা যায়নি
registry-missing = টেমপ্লেট প্যাক তার উইজেটগুলি ঘোষণা করে না
render-timeout = রেন্ডারিং { $seconds } সেকেন্ডের বেশি সময় নিয়েছে এবং থামিয়ে দেওয়া হয়েছে
render-out-of-memory = রেন্ডারিংয়ে { $mib } MiB-এর বেশি মেমরি লেগেছে এবং থামিয়ে দেওয়া হয়েছে
render-worker-lost = রেন্ডারিং প্রসেস থেমে গেছে
render-frame = রেন্ডারিং প্রসেসের বার্তা পড়া যাচ্ছে না
render-page-too-large = এই স্লাইডটি প্রিভিউয়ের জন্য খুব বড় ({ $mib } MiB-এর বেশি, প্রায়ই খুব বড় কোনো ছবির কারণে)। তবু এটি PDF বা PNG হিসেবে এক্সপোর্ট করা যায়।
render-stale = রেন্ডারিং বদলে গেছে; আবার চেষ্টা করুন
render-pack-disabled = “{ $pack }” প্যাকের উইজেট এই সেশনে নিষ্ক্রিয়
registry-schema-remote = উইজেট রেজিস্ট্রি অবৈধ: { $message }
probe-not-probed = এখনও যাচাই করা হয়নি: প্যাকের উইজেটগুলোর জন্য নির্ধারিত সময় শেষ। পরের রেন্ডারে যাচাই করা হবে।
probe-too-many-files = প্যাকে { $max }টির বেশি উইজেট ফাইল আছে: এটি লোড করা হয়নি।
probe-too-large = { $max } KiB-এর চেয়ে বড়: লোড করা হয়নি।

import-unreadable = এই উপস্থাপনাটি পড়া যায়নি
import-not-a-presentation = এই ফাইলটি PowerPoint উপস্থাপনা নয়
import-protected = এই উপস্থাপনাটি এনক্রিপ্ট করা বা পুরোনো .ppt ফরম্যাটে: আগে এটি .pptx হিসেবে সংরক্ষণ করুন
import-too-large = এই উপস্থাপনাটি ইমপোর্ট করার জন্য খুব বড়
import-loss-list-levels = তালিকার স্তরগুলো একটি স্তরে নামানো হয়েছে
import-loss-merged-cells = একত্রিত ঘরগুলো আলাদা করা হয়েছে
import-loss-percent-stacked = ১০০% স্তূপীকৃত চার্ট স্তূপীকৃত হিসেবে দেখানো হয়েছে
import-loss-run-style = { $effect ->
    [underline] আন্ডারলাইন হারিয়েছে
    [strike] কাটা দাগ হারিয়েছে
    [baseline] সুপারস্ক্রিপ্ট বা সাবস্ক্রিপ্ট হারিয়েছে
    [caps] বড় হাতের অক্ষর হারিয়েছে
   *[highlight] হাইলাইট হারিয়েছে
}
import-loss-transform = ঘোরানো, উল্টানো বা ক্রপ হারিয়েছে
import-loss-color-dropped = { $color } রঙের কাছাকাছি কোনো থিম রঙ নেই: থিমের রঙ ব্যবহার করা হয়েছে
import-loss-text-truncated = সীমার মধ্যে রাখতে লেখা ছোট করা হয়েছে
import-loss-table-truncated = সীমার মধ্যে রাখতে টেবিল ছোট করা হয়েছে
import-loss-opaque = { $reason ->
    [media] ভিডিও বা শব্দ ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [ole] এমবেড করা অবজেক্ট ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [math] সমীকরণ ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [smartart] SmartArt ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
   *[unknown] অসমর্থিত অবজেক্ট ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
}
import-loss-unsupported-image = { $format } ফরম্যাটের ছবি ইমপোর্ট করা হয়নি
import-loss-hidden-slide = লুকানো স্লাইড দৃশ্যমান হিসেবে ইমপোর্ট করা হয়েছে
import-loss-effects = ছায়া, প্রতিফলন ও 3D প্রভাব হারিয়েছে
import-loss-series-dropped = পাই চার্ট শুধু একটি সিরিজ দেখায়: { $kept }টি রাখা হয়েছে, { $dropped }টি বাদ দেওয়া হয়েছে
import-loss-repeated-decor = { $slides ->
   [one] { $slides }টি স্লাইডে পুনরাবৃত্ত হেডার বা ফুটার “{ $text }” বাদ দেওয়া হয়েছে: থিম নিজেরটি আঁকে
  *[other] { $slides }টি স্লাইডে পুনরাবৃত্ত হেডার বা ফুটার “{ $text }” বাদ দেওয়া হয়েছে: থিম নিজেরটি আঁকে
}
import-loss-no-slot = বিষয়বস্তু বাদ দেওয়া হয়েছে: বেছে নেওয়া টেমপ্লেটে এর জায়গা নেই
import-warning-effect-ignored = ইফেক্ট ইমপোর্ট করা হয়নি: { $detail }
import-warning-approximated = আনুমানিকভাবে দেখানো হয়েছে: { $detail }
import-warning-unreadable-item = অপাঠযোগ্য আইটেম বাদ দেওয়া হয়েছে: { $detail }
import-warning-unreadable-part = অপাঠযোগ্য অংশ: { $detail }
import-warning-missing-part = অনুপস্থিত অংশ: { $detail }
import-warning-unsupported-chart = অসমর্থিত চার্ট: { $detail }
import-warning-unsupported-image = অসমর্থিত ছবির ফরম্যাট: { $detail }
import-warning-unknown-table-style = অজানা টেবিল স্টাইল: { $detail }
import-warning-unsafe-link = লিঙ্ক প্রত্যাখ্যান করা হয়েছে: { $detail }
import-warning-nesting-too-deep = নেস্টিং খুব গভীর, কেটে দেওয়া হয়েছে: { $detail }
import-warning-font-embedded-unreadable = এমবেড করা ফন্ট পড়া যায়নি: { $detail }
import-warning-truncated = অনেক বেশি সতর্কবার্তা: বাকিগুলো বাদ দেওয়া হয়েছে

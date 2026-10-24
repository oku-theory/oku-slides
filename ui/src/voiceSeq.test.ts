import { expect, test } from "bun:test";
import { bumpVoiceSeq, subscribeVoiceSeq, voiceSeq } from "./voiceSeq";

test("bumpVoiceSeq with a subscriber calls it once", () => {
  let calls = 0;
  const off = subscribeVoiceSeq(() => calls++);
  bumpVoiceSeq();
  off();
  expect(calls).toBe(1);
});

test("bumpVoiceSeq once raises voiceSeq by one", () => {
  const before = voiceSeq();
  bumpVoiceSeq();
  expect(voiceSeq()).toBe(before + 1);
});

test("subscribeVoiceSeq after unsubscribe is not called", () => {
  let calls = 0;
  const off = subscribeVoiceSeq(() => calls++);
  off();
  bumpVoiceSeq();
  expect(calls).toBe(0);
});

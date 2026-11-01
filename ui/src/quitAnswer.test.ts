// Answering the host's quit request: the guards run, then the answer, always once per request.

import { expect, test } from "bun:test";
import { quitAnswerer } from "./quitAnswer";

/** A promise with its resolve and reject, to hold the guards' work open. */
function held() {
  let resolve = () => {};
  let reject = (_: unknown) => {};
  const promise = new Promise<void>((ok, no) => {
    resolve = ok;
    reject = no;
  });
  return { promise, resolve, reject };
}

test("quitAnswerer answers only after the guards ran", async () => {
  const work = held();
  const answers: string[] = [];
  const answer = quitAnswerer(() => work.promise, async () => void answers.push("ready"));
  const done = answer(3000);
  await Promise.resolve();
  expect(answers).toEqual([]);
  work.resolve();
  await done;
  expect(answers).toEqual(["ready"]);
});

test("quitAnswerer with failing guards still answers", async () => {
  const answers: string[] = [];
  const answer = quitAnswerer(() => Promise.reject(new Error("stuck")), async () => void answers.push("ready"));
  await answer(3000);
  expect(answers).toEqual(["ready"]);
});

test("quitAnswerer passes the host's bound to the guards", async () => {
  const bounds: number[] = [];
  const answer = quitAnswerer(async (ms) => void bounds.push(ms), async () => {});
  await answer(1234);
  expect(bounds).toEqual([1234]);
});

test("quitAnswerer second request while running joins it", async () => {
  const work = held();
  let runs = 0;
  let answers = 0;
  const answer = quitAnswerer(
    () => {
      runs++;
      return work.promise;
    },
    async () => void answers++,
  );
  const first = answer(3000);
  const second = answer(3000);
  work.resolve();
  await Promise.all([first, second]);
  expect([runs, answers]).toEqual([1, 1]);
});

test("quitAnswerer with a failing answer does not reject", async () => {
  const answer = quitAnswerer(async () => {}, () => Promise.reject(new Error("gone")));
  await expect(answer(3000)).resolves.toBeUndefined();
});

test("quitAnswerer request after a finished one runs the work again", async () => {
  let runs = 0;
  let answers = 0;
  const answer = quitAnswerer(async () => void runs++, async () => void answers++);
  await answer(3000);
  await answer(3000);
  expect([runs, answers]).toEqual([2, 2]);
});

test("quitAnswerer with hanging work answers once the host's bound passed", async () => {
  const work = held();
  const answers: string[] = [];
  const answer = quitAnswerer(() => work.promise, async () => void answers.push("ready"));
  await answer(20);
  expect(answers).toEqual(["ready"]);
  work.resolve();
});

import { afterAll, expect, test } from "bun:test";
import type { PackInfo } from "./api";
import { setMessages } from "./i18n";
import { packOptionLabel, setTemplatesOp } from "./packOps";
import { resetMessages } from "./test/messages";

afterAll(resetMessages);

const pack = (over: Partial<PackInfo>): PackInfo => ({
  id: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",
  version: "sha256:" + "a".repeat(64),
  name: "Essentiel",
  description: "",
  builtin: true,
  default: false,
  received: false,
  count: 3,
  ...over,
});

test("setTemplatesOp sends the full pack reference", () => {
  expect(setTemplatesOp(pack({}))).toEqual({
    op: "set_templates",
    templates: { id: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10", version: "sha256:" + "a".repeat(64) },
  });
});

test("setTemplatesOp for the default pack still sends id and version, never null", () => {
  expect(setTemplatesOp(pack({ default: true })).templates).toEqual({ id: pack({}).id, version: pack({}).version });
});

test("packOptionLabel tells a received pack from mine and leaves a built-in one bare", () => {
  setMessages("en", ["ui-picker-set-mine = { $name } · mine\nui-picker-set-received = { $name } · received"]);
  expect(packOptionLabel(pack({}))).toBe("Essentiel");
  expect(packOptionLabel(pack({ builtin: false, name: "Charte" }))).toBe("Charte · mine");
  expect(packOptionLabel(pack({ builtin: false, received: true, name: "Charte" }))).toBe("Charte · received");
});

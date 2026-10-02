import { expect, test } from "bun:test";
import type { PackInfo } from "./api";
import { setTemplatesOp } from "./packOps";

const pack = (over: Partial<PackInfo>): PackInfo => ({
  id: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",
  version: "sha256:" + "a".repeat(64),
  name: "Essentiel",
  description: "",
  builtin: true,
  default: false,
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

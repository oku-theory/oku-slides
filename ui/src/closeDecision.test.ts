import { describe, expect, spyOn, test } from "bun:test";
import type { CloseDecision, WorkspaceView } from "./api";
import { confirmFor, redeem } from "./closeDecision";

const view = () => Promise.resolve({ windows: [] } as unknown as WorkspaceView);

describe("closeDecision", () => {
  test("redeem warns when taking the decision fails, then asks", async () => {
    const warn = spyOn(console, "warn").mockImplementation(() => {});
    const take = (): Promise<CloseDecision | null> => Promise.reject(new Error("host gone"));
    expect(await redeem(take, "id", view)).toBeUndefined();
    expect(warn.mock.calls.flat().join(" ")).toContain("host gone");
    warn.mockRestore();
  });

  test("confirmFor warns when the decision cannot be read, then asks", async () => {
    const warn = spyOn(console, "warn").mockImplementation(() => {});
    const decide = () => Promise.reject(new Error("view gone"));
    const asked: string[] = [];
    const confirm = confirmFor(
      async (q) => {
        asked.push(q.message);
        return true;
      },
      decide,
      "t-1",
    );
    expect(await confirm({ message: "discard?", danger: true } as Parameters<typeof confirm>[0])).toBe(true);
    expect(asked).toEqual(["discard?"]);
    expect(warn.mock.calls.flat().join(" ")).toContain("view gone");
    warn.mockRestore();
  });
});

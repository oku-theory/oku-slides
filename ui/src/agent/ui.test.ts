import { describe, expect, test } from "bun:test";
import type { FieldView } from "../api";
import { agentUi } from "./ui";

const field = (owner: string, owner_kind: "slide" | "element", name: string): FieldView => ({
  owner, owner_kind, widget: "card", field: name, kind: "plain", label: null, text: "", empty: false, side: null,
  x: 0, y: 0, w: 0, h: 0, ax: 0, aw: 0, style: null,
});

describe("agentUi", () => {
  test("agentUi with a slide field in edition gives the field without element", () => {
    expect(agentUi("s-1", null, field("s-1", "slide", "title"))).toEqual({ slide: "s-1", element: null, field: "title", editing: true });
  });

  test("agentUi with a card field in edition gives the card and the field", () => {
    expect(agentUi("s-1", "c1", field("c1", "element", "body"))).toEqual({ slide: "s-1", element: "c1", field: "body", editing: true });
  });

  test("agentUi with a field in edition follows the field, not the selection", () => {
    expect(agentUi("s-1", "e-9", field("c1", "element", "body")).element).toBe("c1");
  });

  test("agentUi with a selection only gives the element", () => {
    expect(agentUi("s-1", "c1", null)).toEqual({ slide: "s-1", element: "c1", field: null, editing: false });
  });

  test("agentUi with nothing selected gives the slide only", () => {
    expect(agentUi(null, null, null)).toEqual({ slide: null, element: null, field: null, editing: false });
  });
});

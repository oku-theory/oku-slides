// Assertion partagée : un bouton ne montre qu'un pictogramme, son nom reste accessible.

import { expect } from "bun:test";

/** Vérifie pictogramme seul, `aria-label` non vide et infobulle qui le reprend. */
export function expectIconOnly(button: Element | null, label?: string) {
  expect(button).not.toBeNull();
  const b = button as Element;
  expect(b.textContent?.trim()).toBe("");
  expect(b.querySelector("svg")).not.toBeNull();
  const name = b.getAttribute("aria-label");
  expect(name).toBeTruthy();
  if (label !== undefined) expect(name).toBe(label);
  expect(b.getAttribute("title")?.startsWith(name as string)).toBe(true);
}

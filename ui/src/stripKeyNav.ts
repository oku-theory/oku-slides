// Keyboard moves along a strip of tabs (`role="tablist"`): arrows, Home and End.

/**
 * The index a key moves to in a strip of `count` tabs from `index` (-1: none current), or `null`
 * for any other key or an empty strip.
 *
 * The arrows wrap and are mirrored in right-to-left text; Home and End go to the ends. The
 * caller prevents the key's default and opens and focuses the tab.
 */
export function stripKeyNav(event: { key: string }, index: number, count: number, rtl: boolean): number | null {
  if (count === 0) return null;
  switch (event.key) {
    case "Home":
      return 0;
    case "End":
      return count - 1;
    case "ArrowLeft":
    case "ArrowRight": {
      const step = (event.key === "ArrowRight") !== rtl ? 1 : -1;
      if (index < 0) return step > 0 ? 0 : count - 1;
      return (index + step + count) % count;
    }
    default:
      return null;
  }
}

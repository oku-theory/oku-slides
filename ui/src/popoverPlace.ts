// Places a floating box next to its anchor, inside the viewport.

/**
 * The place of a box `width` wide next to its anchor `cell`, relative to `box`, never past its
 * right edge. Without `height`, the box goes under the cell (a table's popover). With `height`,
 * it flips above the cell when it would pass the bottom, and stays inside `box` vertically.
 */
export function popoverPlace(box: DOMRect, cell: DOMRect, width: number, height?: number): { left: number; top: number } {
  const left = Math.max(0, Math.min(cell.left - box.left, box.width - width));
  const below = cell.bottom - box.top;
  if (height === undefined) return { left, top: below };
  const top = below + height <= box.height ? below : cell.top - box.top - height;
  return { left, top: Math.max(0, Math.min(top, box.height - height)) };
}

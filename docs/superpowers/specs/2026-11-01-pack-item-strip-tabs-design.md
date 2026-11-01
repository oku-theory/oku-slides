# Pack editor: item strip, preview, then Settings and Code tabs

Task t-1419. Supersedes the layout of `2026-10-31-pack-editor-layout-design.md` (sections 1
and the code divider); its slot picker, labels, help and Switch stay.

## Decisions (made on 2026-11-01, user)

1. **Every pack tab** (widgets and layouts) stacks three groups, with air between them:
   - under the header, a **horizontal strip**: Commun when it exists, one thumbnail per item
     with its name under it, then the name field and the add button;
   - the **preview** of the selected item;
   - its **Settings** and **Code** as two tabs, the diagnostics under them.
2. **No left column** in pack tabs: the strip replaces the slim list and the layouts section.
   A theme tab keeps its foldable column.
3. **Tabs replace the toolbar code switch.** The open tab is kept by the host in `code`
   (Settings by default); undo and redo drive the editor while its code shows.
4. **No code divider**: the preview and the tab panel share the height. `code_width` leaves
   `UiState`; an older `state.json` still reads, the field ignored.
5. **Commun** has no settings: its code shows, with no tabs; its widgets in error take the
   preview's place.
6. Alt+Left and Alt+Right move an item along the strip (mirrored right to left).
7. **A widget's thumbnail is the widget alone**, not its slide: its full-slot preview
   (`preview_widget`), cropped to what it draws like the large preview (`croppedSvg`), asked one
   widget at a time and kept until its slide hash or the deck style change. Example values are
   edited on the selected widget, so only its key also follows the pack's trials, and its own
   full-slot preview is reused (`useWidgetThumbs`). A layout's thumbnail stays its slide.

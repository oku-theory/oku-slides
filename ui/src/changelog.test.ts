import { describe, expect, test } from "bun:test";
import { parseChangelog } from "./changelog";

const SAMPLE = `# Changelog

All notable changes to Oku Slides are documented in this file.

## [7.0.0] - 2026-10-27

### Added

- *(platform)* OKU_SLIDES_DEBUG_NETWORK turns on HTTP and TLS debug logs (t-1217)
- *(session,ui)* [**breaking**] themes open in their own tab (t-1156)

### Fixed

- keep an unreadable config aside (t-1216)

## [6.0.0] - 2026-10-26

### Documentation

- *(readme)* document the config.toml file (t-1215)
`;

describe("parseChangelog", () => {
  test("parseChangelog sample lists releases newest first", () => {
    expect(parseChangelog(SAMPLE).map((r) => [r.version, r.date])).toEqual([
      ["7.0.0", "2026-10-27"],
      ["6.0.0", "2026-10-26"],
    ]);
  });

  test("parseChangelog sample keeps groups in order", () => {
    expect(parseChangelog(SAMPLE)[0].groups.map((g) => g.title)).toEqual(["Added", "Fixed"]);
  });

  test("parseChangelog scoped breaking item splits scope and flag", () => {
    expect(parseChangelog(SAMPLE)[0].groups[0].items[1]).toEqual({
      scope: "session,ui",
      breaking: true,
      text: "themes open in their own tab (t-1156)",
    });
  });

  test("parseChangelog item without scope keeps its whole text", () => {
    expect(parseChangelog(SAMPLE)[0].groups[1].items[0]).toEqual({
      scope: null,
      breaking: false,
      text: "keep an unreadable config aside (t-1216)",
    });
  });

  test("parseChangelog header only returns no release", () => {
    expect(parseChangelog("# Changelog\n\nNothing yet.\n")).toEqual([]);
  });
});

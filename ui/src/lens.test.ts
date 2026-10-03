import { describe, expect, test } from "bun:test";
import { backdropLens } from "./lens";

const SAFARI =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
const WKWEBVIEW = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
const WEBKITGTK = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)";
const CHROME =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const WEBVIEW2 =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";

describe("backdropLens", () => {
  test("WKWebView de Tauri sur macOS : pas de lentille sur le fond", () => {
    expect(backdropLens(WKWEBVIEW)).toBe(false);
  });

  test("Safari : pas de lentille sur le fond", () => {
    expect(backdropLens(SAFARI)).toBe(false);
  });

  test("WebKitGTK de Tauri sous Linux : pas de lentille sur le fond", () => {
    expect(backdropLens(WEBKITGTK)).toBe(false);
  });

  test("Chrome : lentille sur le fond", () => {
    expect(backdropLens(CHROME)).toBe(true);
  });

  test("WebView2 de Tauri sous Windows : lentille sur le fond", () => {
    expect(backdropLens(WEBVIEW2)).toBe(true);
  });
});

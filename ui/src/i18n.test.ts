import { afterAll, afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  applyMessages, changeLocale, direction, isolate, loadMessages, locale, onLocaleChange, receiveMessages, setMessages, t,
  tPlain,
} from "./i18n";
import { resetMessages } from "./test/messages";

const en = "ui-hello = Hello { $name }\nui-only-en = Only in English\n";
const fr = [
  "ui-hello = Bonjour { $name }",
  "ui-kind = { $kind ->\n    [text] Texte\n   *[other] { $kind }\n}",
  "ui-size = { NUMBER($value, maximumFractionDigits: 1) } Go",
  "-brand = Oku { $edition }",
  "ui-brand = { -brand(edition: \"Pro\") }",
  "ui-unsaved = Non enregistré { $name }",
  "ui-tab = { $dirty ->\n    [yes] { ui-unsaved }\n   *[no] { $name }\n}",
  "ui-when = Le { $when }",
  "",
].join("\n");

/** `s` between one FSI and one PDI, as `t()` prints a string argument. */
const fsi = (s: string) => `\u2068${s}\u2069`;

// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(resetMessages);

describe("t", () => {
  beforeEach(() => setMessages("fr", [en, fr]));

  test("t isolates a string argument in the chosen language (t-1380)", () => {
    expect(t("ui-hello", { name: "Ada" })).toBe(`Bonjour ${fsi("Ada")}`);
    expect(locale()).toBe("fr");
  });

  test("un message absent de la langue vient de l'anglais", () => {
    expect(t("ui-only-en")).toBe("Only in English");
  });

  test("un identifiant inconnu se voit tel quel", () => {
    expect(t("ui-missing")).toBe("ui-missing");
  });

  test("t strips bidi controls and invisible characters from string arguments (t-959)", () => {
    expect(t("ui-hello", { name: "Ré\u202Ecent\u2066e\u200B\u{E0041}" })).toBe(`Bonjour ${fsi("Récente")}`);
  });

  test("t strips the Hangul fillers and keeps an emoji's variation selector (t-1383)", () => {
    expect(t("ui-hello", { name: "\u3164A\uFFA0\u2764\uFE0F" })).toBe(`Bonjour ${fsi("A\u2764\uFE0F")}`);
  });

  test("t keeps the joiners of an emoji sequence", () => {
    expect(t("ui-hello", { name: "👩\u200D💻" })).toBe(`Bonjour ${fsi("👩\u200D💻")}`);
  });

  test("t turns line breaks and terminal controls in string arguments into spaces (t-1379)", () => {
    expect(t("ui-hello", { name: "a\r\nb\u0085c\u2028d\u2029e\tf\u001Bg\u007Fh" })).toBe(`Bonjour ${fsi("a  b c d e f g h")}`);
  });

  test("t does not nest an argument that already is one isolate, and strips its interior", () => {
    expect(t("ui-hello", { name: isolate("A\u202Eb") })).toBe(`Bonjour ${fsi("Ab")}`);
  });

  test("t keeps each isolated item of a joined list in its own isolate", () => {
    const list = [isolate("\u05D0\u202E"), isolate("B")].join(", ");
    expect(t("ui-hello", { name: list })).toBe(`Bonjour ${fsi(`${fsi("\u05D0")}, ${fsi("B")}`)}`);
  });

  test("t leaves number arguments as they are", () => {
    expect(t("ui-hello", { name: 3 })).toBe("Bonjour 3");
  });

  test("t keeps a right-to-left argument from reordering the sentence (t-1380)", () => {
    expect(t("ui-hello", { name: "\u05E9\u05DC\u05D5\u05DD 2" })).toBe(`Bonjour ${fsi("\u05E9\u05DC\u05D5\u05DD 2")}`);
  });

  test("t matches a selector on the raw string", () => {
    expect(t("ui-kind", { kind: "text" })).toBe("Texte");
  });

  test("t isolates a string printed by a selector's variant", () => {
    expect(t("ui-kind", { kind: "acme/x" })).toBe(fsi("acme/x"));
  });

  test("t leaves a number passed to NUMBER() unisolated", () => {
    expect(t("ui-size", { value: 1.25 })).toBe("1,3 Go");
  });

  test("t isolates a term's parameter where the term prints it", () => {
    expect(t("ui-brand")).toBe(`Oku ${fsi("Pro")}`);
  });

  test("t isolates the arguments of a message referenced from a variant", () => {
    expect(t("ui-tab", { dirty: "yes", name: "A" })).toBe(`Non enregistré ${fsi("A")}`);
  });

  test("t leaves a date argument unisolated", () => {
    expect(t("ui-when", { when: new Date(0) })).not.toContain("\u2068");
  });

  test("t isolates a string argument of a message overridden by a later resource", () => {
    setMessages("fr", [en, fr, "ui-hello = Salut { $name }\n"]);
    expect(t("ui-hello", { name: "Ada" })).toBe(`Salut ${fsi("Ada")}`);
  });

  test("tPlain formats the message without isolation marks", () => {
    expect(tPlain("ui-hello", { name: isolate("Ada") })).toBe("Bonjour Ada");
  });
});

test("avant le chargement, t rend l'identifiant", () => {
  setMessages("en", []);
  expect(t("ui-hello")).toBe("ui-hello");
});

test("direction : de droite à gauche pour l'arabe, de gauche à droite sinon", () => {
  expect(direction("ar")).toBe("rtl");
  expect(direction("ar-EG")).toBe("rtl");
  expect(direction("fr")).toBe("ltr");
  expect(direction("zh-Hant")).toBe("ltr");
});

describe("choix de la langue", () => {
  test("applyMessages retient le choix, puis prévient", () => {
    let told = 0;
    const stop = onLocaleChange(() => told++);
    applyMessages({ locale: "de", resources: [en], chosen: "de" });
    stop();
    expect(locale()).toBe("de");
    expect(told).toBe(1);
    // Le choix est retenu : les mêmes messages reçus ensuite ne changent rien.
    expect(receiveMessages({ locale: "de", resources: [en], chosen: "de" })).toBe(false);
  });

  test("receiveMessages applique la langue choisie dans une autre fenêtre et prévient", () => {
    applyMessages({ locale: "fr", resources: [en, fr], chosen: "fr" });
    let told = 0;
    const stop = onLocaleChange(() => told++);
    const applied = receiveMessages({ locale: "de", resources: [en], chosen: "de" });
    stop();
    expect(applied).toBe(true);
    expect(locale()).toBe("de");
    expect(told).toBe(1);
  });

  test("receiveMessages ignore ses propres messages : la fenêtre qui a choisi n'est pas remontée deux fois", () => {
    applyMessages({ locale: "de", resources: [en], chosen: "de" });
    let told = 0;
    const stop = onLocaleChange(() => told++);
    const applied = receiveMessages({ locale: "de", resources: [en], chosen: "de" });
    stop();
    expect(applied).toBe(false);
    expect(told).toBe(0);
  });

  test("receiveMessages suit le retour à la langue du système, même si elle ne change pas", () => {
    applyMessages({ locale: "fr", resources: [en, fr], chosen: "fr" });
    let told = 0;
    const stop = onLocaleChange(() => told++);
    expect(receiveMessages({ locale: "fr", resources: [en, fr], chosen: null })).toBe(true);
    stop();
    expect(told).toBe(1);
    expect(receiveMessages({ locale: "fr", resources: [en, fr], chosen: null })).toBe(false);
  });

  describe("changeLocale, que la diffusion de l'hôte arrive avant ou après sa réponse", () => {
    const de = { locale: "de", resources: [en], chosen: "de" };
    const realFetch = globalThis.fetch;
    afterEach(() => {
      globalThis.fetch = realFetch;
    });
    /** L'hôte répond `de` à `set_locale`, après avoir (peut-être) diffusé `messages`. */
    function hostAnswers(broadcastFirst: boolean) {
      globalThis.fetch = (async () => {
        if (broadcastFirst) receiveMessages(de);
        return new Response(JSON.stringify(de), { status: 200 });
      }) as unknown as typeof fetch;
    }

    const fr = { locale: "fr", resources: [en], chosen: "fr" };
    /** Un hôte dont chaque réponse attend qu'on la libère, dans l'ordre qu'on veut. */
    function slowHost() {
      const pending: { body: unknown; release: () => void }[] = [];
      globalThis.fetch = ((_url: string, init?: RequestInit) => {
        const req = JSON.parse(String(init?.body));
        const body = req.cmd === "set_locale" ? (req.locale === "de" ? de : fr) : fr;
        return new Promise<Response>((resolve) => {
          pending.push({ body, release: () => resolve(new Response(JSON.stringify(body), { status: 200 })) });
        });
      }) as unknown as typeof fetch;
      return pending;
    }

    test("deux choix rapides : une réponse arrivée en retard n'écrase pas le dernier choix", async () => {
      applyMessages({ locale: "en", resources: [en], chosen: null });
      const pending = slowHost();
      const first = changeLocale("fr");
      const second = changeLocale("de");
      pending[1].release();
      await second;
      pending[0].release();
      await first;
      expect(locale()).toBe("de");
    });

    test("une langue diffusée pendant le premier chargement n'est pas écrasée par sa réponse", async () => {
      applyMessages({ locale: "en", resources: [en], chosen: null });
      const pending = slowHost();
      const load = loadMessages();
      receiveMessages(de); // une autre fenêtre choisit pendant que la lecture est en route
      pending[0].release(); // la lecture, partie avant le choix, répond `fr`
      await load;
      expect(locale()).toBe("de");
    });

    for (const broadcastFirst of [true, false]) {
      test(`remonte l'interface une seule fois (diffusion ${broadcastFirst ? "avant" : "absente ou après"} la réponse)`, async () => {
        applyMessages({ locale: "fr", resources: [en, fr], chosen: "fr" });
        hostAnswers(broadcastFirst);
        let told = 0;
        const stop = onLocaleChange(() => told++);
        await changeLocale("de");
        receiveMessages(de); // une diffusion arrivée après la réponse : sans effet
        stop();
        expect(locale()).toBe("de");
        expect(told).toBe(1);
      });
    }
  });
});

import { afterAll, afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  applyMessages, changeLocale, direction, isolate, loadMessages, locale, onLocaleChange, receiveMessages, setMessages, t,
} from "./i18n";
import { resetMessages } from "./test/messages";

const en = "ui-hello = Hello { $name }\nui-only-en = Only in English\n";
const fr = "ui-hello = Bonjour { $name }\n";

// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(resetMessages);

describe("t", () => {
  beforeEach(() => setMessages("fr", [en, fr]));

  test("dit le message dans la langue choisie, avec ses paramètres, sans marques d'isolement", () => {
    expect(t("ui-hello", { name: "Ada" })).toBe("Bonjour Ada");
    expect(locale()).toBe("fr");
  });

  test("un message absent de la langue vient de l'anglais", () => {
    expect(t("ui-only-en")).toBe("Only in English");
  });

  test("un identifiant inconnu se voit tel quel", () => {
    expect(t("ui-missing")).toBe("ui-missing");
  });

  test("t strips bidi controls and invisible characters from string arguments (t-959)", () => {
    expect(t("ui-hello", { name: "Ré\u202Ecent\u2066e\u200B\u{E0041}" })).toBe("Bonjour Récente");
  });

  test("t strips the Hangul fillers and keeps an emoji's variation selector (t-1383)", () => {
    expect(t("ui-hello", { name: "\u3164A\uFFA0\u2764\uFE0F" })).toBe("Bonjour A\u2764\uFE0F");
  });

  test("t keeps the joiners of an emoji sequence", () => {
    expect(t("ui-hello", { name: "👩\u200D💻" })).toBe("Bonjour 👩\u200D💻");
  });

  test("t turns line breaks and terminal controls in string arguments into spaces (t-1379)", () => {
    expect(t("ui-hello", { name: "a\r\nb\u0085c\u2028d\u2029e\tf\u001Bg\u007Fh" })).toBe("Bonjour a  b c d e f g h");
  });

  test("t keeps the one isolation pair of an isolated argument and strips its interior", () => {
    expect(t("ui-hello", { name: isolate("A\u202Eb") })).toBe("Bonjour \u2068Ab\u2069");
  });

  test("t keeps each isolated item of a joined list in its own isolate", () => {
    const list = [isolate("\u05D0\u202E"), isolate("B")].join(", ");
    expect(t("ui-hello", { name: list })).toBe("Bonjour \u2068\u05D0\u2069, \u2068B\u2069");
  });

  test("t leaves number arguments as they are", () => {
    expect(t("ui-hello", { name: 3 })).toBe("Bonjour 3");
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

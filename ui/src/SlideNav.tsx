/** Boutons discrets slide précédente / suivante, de part et d'autre de la scène. */
import { t } from "./i18n";
import { Icon } from "./icons";
import { slideStep } from "./slideKeys";

export default function SlideNav({ slides, current, onShow }: {
  slides: readonly { id: string }[];
  /** Slide montrée ; `null` avant la première. */
  current: string | null;
  onShow: (id: string) => void;
}) {
  const index = slides.findIndex((s) => s.id === current);
  const button = (delta: -1 | 1) => {
    const to = slideStep(index, delta, slides.length);
    const label = t(delta < 0 ? "ui-slide-previous" : "ui-slide-next");
    return (
      <button
        className={`token ghost slide-nav ${delta < 0 ? "previous" : "next"}`}
        aria-label={label}
        title={label}
        disabled={to === null}
        // La scène désélectionne au pointeur : le clic n'y descend pas.
        onPointerDown={(e) => e.stopPropagation()}
        onClick={() => to !== null && onShow(slides[to].id)}
      >
        {delta < 0 ? Icon.previous : Icon.next}
      </button>
    );
  };
  return <>{button(-1)}{button(1)}</>;
}

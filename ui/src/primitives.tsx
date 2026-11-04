// Primitives partagées par le catalogue A2UI et l'UI maison. Bouton, bascule à pictogramme,
// liste déroulante et champ libellé : à utiliser plutôt qu'un `.btn` ou un `select.select` écrit
// à la main. Elles ne posent que les classes et attributs communs ; l'apparence reste dans
// styles.css et a2ui.css.

import { useId } from "react";
import type { ButtonHTMLAttributes, ReactNode, Ref, SelectHTMLAttributes } from "react";

/** Les classes non vides, séparées par une espace. */
const classes = (...names: (string | false | undefined)[]) => names.filter(Boolean).join(" ");

/** Un bouton de texte `.btn`. */
export function Button({ raised, primary, ghost, danger, className, type = "button", ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & {
  /** Un objet posé sur la table, qui se soulève au survol (`.object`). */
  raised?: boolean;
  /** L'action principale. */
  primary?: boolean;
  /** Plat, sans fond. */
  ghost?: boolean;
  /** Une action qui engage (non annulable, permission permanente). */
  danger?: boolean;
}) {
  const name = classes(raised && "object", "btn", primary && "primary", ghost && "ghost", danger && "danger", className);
  return <button type={type} className={name} {...rest} />;
}

/**
 * Un bouton à pictogramme seul, sans état.
 *
 * Fermé à dessein : son libellé est à la fois son nom (`aria-label`) et son infobulle.
 */
export function IconButton({ label, icon, className, ...rest }: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-label" | "title" | "children" | "type"> & {
  /** Nom accessible et infobulle. */
  label: string;
  /** Le pictogramme, seul contenu du bouton. */
  icon: ReactNode;
  /** The button element, to give it the focus. */
  ref?: Ref<HTMLButtonElement>;
}) {
  return (
    <button type="button" aria-label={label} title={label} className={className} {...rest}>
      {icon}
    </button>
  );
}

/**
 * Un bouton à pictogramme seul, enfoncé ou non.
 *
 * Fermé à dessein : son libellé est à la fois son nom (`aria-label`) et son infobulle.
 */
export function IconToggle({ label, pressed, onToggle, icon, className, disabled, keepFocus }: {
  /** Nom accessible et infobulle. */
  label: string;
  /** Enfoncé : `aria-pressed` et la classe `on`. */
  pressed: boolean;
  onToggle: () => void;
  /** Le pictogramme, seul contenu du bouton. */
  icon: ReactNode;
  className?: string;
  disabled?: boolean;
  /** A click leaves the focus where it is: a text being edited stays open. */
  keepFocus?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={pressed}
      className={classes(className, pressed && "on") || undefined}
      disabled={disabled}
      onMouseDown={keepFocus ? (e) => e.preventDefault() : undefined}
      onClick={onToggle}
    >
      {icon}
    </button>
  );
}

/** Une option d'une liste déroulante ; sa valeur est unique dans la liste. */
interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

/** Une liste déroulante `.select` ; `onChange` reçoit la valeur choisie et la liste. */
export function Select({ value, options, onChange, ...rest }: Omit<
  SelectHTMLAttributes<HTMLSelectElement>,
  "value" | "onChange" | "className" | "children"
> & {
  value: string;
  options: readonly SelectOption[];
  onChange: (value: string, select: HTMLSelectElement) => void;
}) {
  return (
    <select {...rest} className="select" value={value} onChange={(e) => onChange(e.target.value, e.currentTarget)}>
      {options.map((o) => <option key={o.value} value={o.value} disabled={o.disabled}>{o.label}</option>)}
    </select>
  );
}

/** Les classes de chaque variante de `Field` : le champ, puis son libellé. */
const FIELD = {
  /** Champ d'une carte A2UI. */
  note: ["note-field", "a2ui-hint"],
  /** Rangée d'un panneau de réglages. */
  row: ["settings-row", "caption"],
  /** Rangée en ligne : libellé à gauche, contrôle à droite (sélecteur de thème). */
  menu: ["menu-theme", "caption"],
} as const;

/** A field's help line and the id its control cites in `aria-describedby`. */
export interface FieldHelp {
  /** The help element's id; absent without help. */
  describedBy: string | undefined;
  /** The muted help line, to place under the control; `null` without help. */
  node: ReactNode;
}

/**
 * The help line of a field, tied to its control for screen readers.
 *
 * The control takes `describedBy` as its `aria-describedby`; `node` goes after the field, outside
 * its `<label>`, so the help is not read as part of the label.
 */
export function useFieldHelp(help: string | undefined): FieldHelp {
  const id = useId();
  if (!help) return { describedBy: undefined, node: null };
  return { describedBy: id, node: <span id={id} className="a2ui-hint field-help">{help}</span> };
}

/**
 * A labelled field: the label, then the control; `as="div"` when it holds several controls.
 *
 * `help`, the `node` of `useFieldHelp`, follows the field.
 */
export function Field({ variant, label, as = "label", help, children }: {
  variant: keyof typeof FIELD;
  label: ReactNode;
  as?: "label" | "div";
  help?: ReactNode;
  children: ReactNode;
}) {
  const [outer, hint] = FIELD[variant];
  const Tag = as;
  return (
    <>
      <Tag className={outer}>
        <span className={hint}>{label}</span>
        {children}
      </Tag>
      {help}
    </>
  );
}

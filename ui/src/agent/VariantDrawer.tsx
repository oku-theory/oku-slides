// Tiroir des variantes d'une slide, sous la scène.

import { t } from "../i18n";
import type { SurfaceProps } from "./AgentSurfaceBox";
import { AgentSurfaceBox } from "./AgentSurfaceBox";

/** Le tiroir : les variantes rendues et le rappel des touches. */
export function VariantDrawer(props: SurfaceProps) {
  return <AgentSurfaceBox {...props} className="variant-drawer" role="dialog" label={t("ui-agent-variants")} caption={t("ui-agent-drawer-keys")} />;
}

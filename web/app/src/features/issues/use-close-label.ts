import { t } from "@bugaboo/messages";
import { useLayout, useUpright } from "@bugaboo/ui";

/**
 * What the close button of the side panel is called. Where the issue takes the whole screen (a
 * phone, a tablet held upright) it leads back to the list; elsewhere the panel's own name stands.
 */
export function useCloseLabel(): string | undefined {
  const layout = useLayout();
  const upright = useUpright();
  return layout === "phone" || (layout === "tablet" && upright) ? t("focus.back") : undefined;
}

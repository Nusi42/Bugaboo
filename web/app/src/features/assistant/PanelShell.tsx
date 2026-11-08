import { t } from "@bugaboo/messages";
import { Overlay, overlayEdge, partOf, part, useLayout } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Dialog } from "react-aria-components";

import { useAssistantDocked, useEscape } from "../shell/index.ts";

/**
 * Where the panel stands: beside the page on a computer, which the page does not dim and which stays
 * usable (UX §4.12); a layer from the end on a tablet; a whole screen on a phone. Esc and the press
 * outside a layer ask to close, and the panel decides whether to ask first.
 */
export function PanelShell({
  onClose,
  children,
}: {
  readonly onClose: () => void;
  readonly children: ReactNode;
}) {
  const layout = useLayout();
  const docked = useAssistantDocked();
  const dock = useEscape<HTMLDivElement>(onClose);
  if (docked) {
    return (
      <div ref={dock} {...partOf(part.assistantDock)}>
        <Dialog aria-label={t("assistant.title")} {...partOf(part.assistantDockBody)}>
          {children}
        </Dialog>
      </div>
    );
  }
  return (
    <Overlay
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={t("assistant.title")}
      from={layout === "phone" ? overlayEdge.full : overlayEdge.end}
    >
      {children}
    </Overlay>
  );
}

import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, overlayEdge, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { IssueLegend } from "./IssueLegend.tsx";

/**
 * The small *Key* button where the legend has no room beside the toolbar (a phone, or a list with
 * an issue open beside it): it opens the legend in a sheet from the bottom (UX §4.2, M5 exit D2).
 */
export function LegendButton() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("issues.key")}
      </Button>
      <Overlay
        isOpen={open}
        onOpenChange={setOpen}
        label={t("issues.legend")}
        from={overlayEdge.bottom}
      >
        <div {...partOf(part.searchPanel, { variants: ["sheet"] })}>
          <IssueLegend />
          <div {...partOf(part.searchPanelActions)}>
            <Button
              look={buttonLook.primary}
              onPress={() => {
                setOpen(false);
              }}
            >
              {t("common.close")}
            </Button>
          </div>
        </div>
      </Overlay>
    </>
  );
}

import { t } from "@bugaboo/messages";
import { BottomSheet, Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import { IssueLegend } from "./IssueLegend.tsx";

/**
 * The small *Legend* button where the legend has no room beside the toolbar (a phone, or a list with
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
        {t("issues.legend-button")}
      </Button>
      <BottomSheet
        isOpen={open}
        onOpenChange={setOpen}
        label={t("issues.legend")}
        closeLabel={t("common.close")}
      >
        <IssueLegend />
      </BottomSheet>
    </>
  );
}

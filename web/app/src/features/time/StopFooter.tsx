import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { askTime } from "./time-asks.ts";

export interface StopFooterProps {
  readonly onSave: () => void;
  readonly isPending: boolean;
}

/** *Throw away* and *Save time*, the buttons of the window that stops the timer; the main one stands at the right (R6). */
export function StopFooter({ onSave, isPending }: StopFooterProps) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button
        look={buttonLook.quietDanger}
        onPress={() => {
          askTime({ kind: "throw-away" });
        }}
      >
        {t("time.timer.throw-away")}
      </Button>
      <Button look={buttonLook.primary} isPending={isPending} onPress={onSave}>
        {t("time.stop.save")}
      </Button>
    </div>
  );
}

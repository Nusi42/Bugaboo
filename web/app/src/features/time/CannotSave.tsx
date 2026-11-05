import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { throwAwayTimer } from "./timer-requests.ts";

/**
 * A timer whose time cannot be saved (its issue is not seen any more, or time cannot be booked
 * there): the reason in words, and *Throw away* as the way out.
 */
export function CannotSave({ why, version }: { readonly why: string; readonly version: number }) {
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          closeTimeAsk();
        }
      }}
      title={t("time.stop.cannot-title")}
      text={why}
      confirmLabel={t("time.timer.throw-away")}
      cancelLabel={t("common.close")}
      confirmLook={buttonLook.danger}
      onConfirm={() => {
        throwAwayTimer(version).catch((error: unknown) => {
          tellTime(messageOf(error), "danger");
        });
      }}
    />
  );
}

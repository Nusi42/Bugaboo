import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  part,
  partOf,
  useHoldWhileReading,
} from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePinning, type PinTarget } from "./pins.ts";

/**
 * *Pin to sidebar* or *Unpin* beside a project, a saved search or a dashboard (CFG-05): it says what it does once the
 * person's pins are read, and a refusal (the limit of 30) is said under it. Both words stand in the button, the one
 * that is not meant unseen, so the button keeps its width when it turns round (UX §6.4). A page that holds its first
 * read waits for the pins too.
 */
export function PinButton(props: PinTarget) {
  const pinning = usePinning(props);
  // Inside a hold the page waits for the pins, so the button does not come in later and push what stands after it.
  useHoldWhileReading(!pinning.known);
  if (!pinning.known) {
    return null;
  }
  const now = pinning.pinned ? t("pins.unpin") : t("pins.pin");
  const other = pinning.pinned ? t("pins.pin") : t("pins.unpin");
  return (
    <>
      <Button look={buttonLook.quiet} isPending={pinning.pending} onPress={pinning.turn}>
        <span {...partOf(part.pinLabel)}>
          <span {...partOf(part.pinLabelWord)}>{now}</span>
          <span {...partOf(part.pinLabelWord, { states: ["spare"] })} aria-hidden>
            {other}
          </span>
        </span>
      </Button>
      {pinning.problem === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(pinning.problem)}</Alert>
      )}
    </>
  );
}

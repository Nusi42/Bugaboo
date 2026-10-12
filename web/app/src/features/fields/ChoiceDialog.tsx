import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import type { OwnField } from "./fields.ts";

export interface ChoiceDialogProps {
  readonly field: OwnField;
  /** The inside of the dialog; `done` closes it. Drawn only while it is open, so nothing is asked before. */
  readonly children: (done: () => void) => ReactNode;
}

/** A button *Choose* that opens a dialog with the choices of a field (a short task: UX §4.4). */
export function ChoiceDialog({ field, children }: ChoiceDialogProps) {
  const [open, setOpen] = useState(false);
  const close = () => {
    setOpen(false);
  };
  return (
    <>
      <Button
        aria-label={t("fields.choose-title", { name: field.name })}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("fields.choose")}
      </Button>
      <Overlay
        isOpen={open}
        onOpenChange={setOpen}
        label={t("fields.choose-title", { name: field.name })}
      >
        {open ? (
          <div {...partOf(part.stack)}>
            {children(close)}
            <Button look={buttonLook.primary} onPress={close}>
              {t("common.done")}
            </Button>
          </div>
        ) : null}
      </Overlay>
    </>
  );
}

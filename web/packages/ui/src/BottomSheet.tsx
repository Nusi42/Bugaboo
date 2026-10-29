import type { ReactNode } from "react";

import { Button } from "./Button.tsx";
import { Overlay } from "./Overlay.tsx";
import { partOf } from "./parts.ts";
import { buttonLook, overlayEdge } from "./props.ts";
import { part } from "./tokens.ts";

export interface BottomSheetProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  /** The name of the sheet, for screen readers. */
  readonly label: string;
  /** The title at the top of the sheet; none where the content says what it is. */
  readonly title?: string;
  /** What is done with the content (*Save search*), next to the button that closes the sheet. */
  readonly footer?: ReactNode;
  /** What the button at the foot says (*Close*, *Done*). */
  readonly closeLabel: string;
  /** The sheet has the height of the sheet of the filters, so it does not jump as its content changes. */
  readonly tall?: boolean;
  readonly children: ReactNode;
}

/** A sheet from the bottom of a phone (UX §3.3): its content, and the main button that closes it at its foot. */
export function BottomSheet({
  isOpen,
  onOpenChange,
  label,
  title,
  footer,
  closeLabel,
  tall = false,
  children,
}: BottomSheetProps) {
  return (
    <Overlay isOpen={isOpen} onOpenChange={onOpenChange} label={label} from={overlayEdge.bottom}>
      <div {...partOf(part.bottomSheet, { variants: [tall && "tall"] })}>
        {title === undefined ? null : <h2 {...partOf(part.bottomSheetTitle)}>{title}</h2>}
        {children}
        <div {...partOf(part.bottomSheetActions)}>
          {footer}
          <Button
            look={buttonLook.primary}
            onPress={() => {
              onOpenChange(false);
            }}
          >
            {closeLabel}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}

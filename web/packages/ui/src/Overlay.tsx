import type { ReactNode } from "react";
import { Dialog, Modal, ModalOverlay } from "react-aria-components";

import { partOf } from "./parts.ts";
import { overlayEdge, type OverlayEdge } from "./props.ts";
import { part } from "./tokens.ts";

export interface OverlayProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  /** The name of what opens, for screen readers: "Quick search", "Main menu". */
  readonly label: string;
  /**
   * Where it opens from: the `bottom` edge (a sheet on a phone), the `start` edge (the menu of a
   * tablet), the `top` edge, or over the middle (the default).
   */
  readonly from?: OverlayEdge;
  readonly children: ReactNode;
}

/**
 * Something that opens over the page and takes the focus: the menu of a tablet, the sheets of a
 * phone (they replace small pop-ups, UX §3.3), quick search and the list of shortcuts. The focus
 * stays inside, Esc and a press outside close it, and the focus goes back to where it was.
 */
export function Overlay({
  isOpen,
  onOpenChange,
  label,
  from = overlayEdge.center,
  children,
}: OverlayProps) {
  return (
    <ModalOverlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable
      {...partOf(part.overlay, { variants: [from] })}
    >
      <Modal {...partOf(part.overlayModal, { variants: [from] })}>
        <Dialog aria-label={label} {...partOf(part.overlayDialog)}>
          {children}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}

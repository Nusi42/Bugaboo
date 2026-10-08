import type { ReactNode } from "react";
import { Dialog, Modal, ModalOverlay } from "react-aria-components";

import { cx } from "./cx.ts";
import styles from "./Overlay.module.css";
import { overlayEdge, type OverlayEdge } from "./props.ts";

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
  const placement = styles[from];
  return (
    <ModalOverlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable
      className={cx(styles.overlay, placement)}
    >
      <Modal className={cx(styles.modal, placement)}>
        <Dialog aria-label={label} className={cx(styles.dialog)}>
          {children}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}

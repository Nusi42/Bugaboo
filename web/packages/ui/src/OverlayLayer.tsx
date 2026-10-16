import type { ReactNode } from "react";
import { Modal, ModalOverlay } from "react-aria-components";

import { partOf } from "./parts.ts";
import type { OverlayEdge } from "./props.ts";
import { part } from "./tokens.ts";
import type { WindowElement } from "./window-size.ts";

export interface OverlayLayerProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  /** Where it opens from: the variant of the overlay and of its modal. */
  readonly from: OverlayEdge;
  /** The layer fills the page (a maximized window). */
  readonly maximized?: boolean;
  /** A press outside closes it. A window does not: its form keeps what was typed. */
  readonly isDismissable: boolean;
  /** The element that is the box of the layer, for what must size it. */
  readonly modalRef?: (element: WindowElement | null) => void;
  readonly children: ReactNode;
}

/**
 * The one layer over the page that every overlay is drawn in: the dimmed page, the box and the
 * way it opens (UX §3.3, §6.3). `Overlay` puts a dialog in it, and `Window` its own frame.
 */
export function OverlayLayer({
  isOpen,
  onOpenChange,
  from,
  maximized = false,
  isDismissable,
  modalRef,
  children,
}: OverlayLayerProps) {
  const states = [maximized && "maximized"] as const;
  return (
    <ModalOverlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable={isDismissable}
      {...partOf(part.overlay, { variants: [from], states })}
    >
      <Modal ref={modalRef} {...partOf(part.overlayModal, { variants: [from], states })}>
        {children}
      </Modal>
    </ModalOverlay>
  );
}

import { useEffect, useRef, type ReactNode } from "react";
import { Dialog } from "react-aria-components";

import { partOf } from "./parts.ts";
import { OverlayLayer } from "./OverlayLayer.tsx";
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

/** The dialog of an overlay: it starts at its top, where the title is, however the focus moved inside it as it opened. */
function Content({ label, children }: { readonly label: string; readonly children: ReactNode }) {
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      dialog.current?.scrollTo({ top: 0 });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, []);
  return (
    <Dialog ref={dialog} aria-label={label} {...partOf(part.overlayDialog)}>
      {children}
    </Dialog>
  );
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
    <OverlayLayer isOpen={isOpen} onOpenChange={onOpenChange} from={from} isDismissable>
      <Content label={label}>{children}</Content>
    </OverlayLayer>
  );
}

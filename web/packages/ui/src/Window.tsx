import type { ReactNode } from "react";

import { OverlayLayer } from "./OverlayLayer.tsx";
import { overlayEdge, windowState } from "./props.ts";
import { useWindow } from "./use-window.ts";
import { WindowDock } from "./WindowDock.tsx";
import { WindowFrame, type WindowLabels } from "./WindowFrame.tsx";
import type { WindowSize } from "./window-size.ts";

export interface WindowProps {
  readonly isOpen: boolean;
  /** Called with `false` when the window closes: *Close*, *Back* or Esc. The draft is the screen's to keep. */
  readonly onOpenChange: (isOpen: boolean) => void;
  readonly title: string;
  readonly labels: WindowLabels;
  /** The buttons of the form; the main one stands at the right (R6). */
  readonly footer?: ReactNode;
  /** The size the person gave this form before (760 wide at first). */
  readonly size?: WindowSize | undefined;
  /** Called with the size the window ended with after a drag or a key, to be remembered. */
  readonly onResized?: ((size: WindowSize) => void) | undefined;
  readonly children: ReactNode;
}

/**
 * A form opens as a window over the page (UX §6.3, R10), in the one overlay of the shell. It has
 * *Minimize*, *Maximize* or *Restore down*, and *Close*; it is resized at its edges and corners
 * with the pointer and with Ctrl and Alt and the arrow keys. Minimized, it becomes a bar at the
 * bottom of the page; its form is drawn again when it opens, so the screen keeps the draft, as it
 * does when the window is closed. The focus stays inside while it is open, and Esc closes it. On a
 * phone it is a full screen with *Back*, without resizing. A question is not a window
 * (`ConfirmDialog`).
 */
export function Window(props: WindowProps) {
  const { isOpen, onOpenChange, title, labels, footer, size, onResized, children } = props;
  const { placed, ...shape } = useWindow(isOpen, size, onResized);
  return (
    <>
      {isOpen && shape.minimized ? (
        <WindowDock title={title} note={labels.draftKept} onRestore={shape.restore} />
      ) : null}
      <OverlayLayer
        isOpen={isOpen && !shape.minimized}
        onOpenChange={onOpenChange}
        from={shape.state === windowState.full ? overlayEdge.full : overlayEdge.window}
        maximized={shape.state === windowState.maximized}
        isDismissable={false}
        modalRef={placed}
      >
        <WindowFrame
          title={title}
          labels={labels}
          state={shape.state}
          footer={footer}
          resize={shape.resize}
          onMinimize={shape.minimize}
          onToggleMaximize={shape.toggleMaximize}
          onClose={() => {
            onOpenChange(false);
          }}
        >
          {children}
        </WindowFrame>
      </OverlayLayer>
    </>
  );
}

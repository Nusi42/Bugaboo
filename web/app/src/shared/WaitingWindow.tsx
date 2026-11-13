import { Window } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { useWindowProps } from "./use-window-props.ts";
import type { WindowForm } from "./window-forms.ts";

export interface WaitingWindowProps {
  /** The form of the window that is on its way: its remembered size is the size this one has. */
  readonly form: WindowForm;
  readonly title: ReactNode;
  readonly onClose: () => void;
}

/**
 * What stands in for a window while its code is fetched (it is loaded when it is first opened,
 * coding-standards §6): the window itself, waiting for its content (UX §6.1). The page is dimmed at
 * once and Esc closes it; after 0.3 s grey shapes show at the top. When the code arrives, the real
 * window takes over where this one was (`Window` hands over how far it got), so nothing jumps. Put it
 * as the `fallback` of the `Suspense` around a window that is loaded later.
 */
export function WaitingWindow({ form, title, onClose }: WaitingWindowProps) {
  const windowProps = useWindowProps(form);
  return (
    <Window
      {...windowProps}
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      title={title}
      ready={false}
    >
      {null}
    </Window>
  );
}

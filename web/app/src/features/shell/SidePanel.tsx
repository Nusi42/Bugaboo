import { t } from "@bugaboo/messages";
import { IconButton, iconName, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { useEscape } from "./use-escape.ts";
import { useTakeFocus } from "./use-take-focus.ts";

export interface SidePanelProps {
  /** What the panel shows: the name of the item that is open. */
  readonly title: string;
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/**
 * The panel at the side of a list (UX §3.4, §5.1): a simple item opens here and the list stays in
 * view. It takes the focus when it opens, closes with Esc or its button, and gives the focus back
 * to what had it before.
 */
export function SidePanel({ title, onClose, children }: SidePanelProps) {
  const heading = useTakeFocus<HTMLHeadingElement>();
  const panel = useEscape<HTMLElement>(onClose);
  return (
    <aside ref={panel} {...partOf(part.sidePanel)} aria-label={title}>
      <div {...partOf(part.sidePanelHead)}>
        <h2 ref={heading} tabIndex={-1} {...partOf(part.sidePanelTitle)}>
          {title}
        </h2>
        <IconButton icon={iconName.close} label={t("panel.close")} onPress={onClose} />
      </div>
      <div {...partOf(part.stack)}>{children}</div>
    </aside>
  );
}

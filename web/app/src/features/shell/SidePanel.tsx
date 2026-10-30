import { t } from "@bugaboo/messages";
import { IconButton, iconName, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { useEscape } from "./use-escape.ts";
import { useTakeFocus } from "./use-take-focus.ts";

export interface SidePanelProps {
  /** The name of the item that is open: the name of the panel for screen readers. */
  readonly title: string;
  /** What stands in the heading instead of the plain name, such as a title that is changed in place. */
  readonly heading?: ReactNode;
  readonly onClose: () => void;
  /** What the close button is called; where the panel takes the whole screen it is *Back to list*. */
  readonly closeLabel?: string;
  readonly children: ReactNode;
}

/**
 * The panel at the side of a list (UX §3.4, §5.1): a simple item opens here and the list stays in
 * view. Beside a list it is as high as the window at most, and its body scrolls under its head
 * (L7). It takes the focus when it opens, closes with Esc or its button, and gives the focus back
 * to what had it before.
 */
export function SidePanel({ title, heading, onClose, closeLabel, children }: SidePanelProps) {
  const titleBox = useTakeFocus<HTMLHeadingElement>();
  const panel = useEscape<HTMLElement>(onClose);
  return (
    <aside ref={panel} {...partOf(part.sidePanel)} aria-label={title}>
      <div {...partOf(part.sidePanelHead)}>
        <h2 ref={titleBox} tabIndex={-1} {...partOf(part.sidePanelTitle)}>
          {heading ?? title}
        </h2>
        <IconButton
          icon={closeLabel === undefined ? iconName.close : iconName.back}
          label={closeLabel ?? t("panel.close")}
          onPress={onClose}
        />
      </div>
      <div {...partOf(part.sidePanelBody)}>
        <div {...partOf(part.stack)}>{children}</div>
      </div>
    </aside>
  );
}

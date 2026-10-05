import { t } from "@bugaboo/messages";
import { cx, IconButton, iconName } from "@bugaboo/ui";
import type { ReactNode } from "react";

import styles from "./SidePanel.module.css";
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
    <aside ref={panel} className={cx(styles.panel)} aria-label={title}>
      <div className={styles.head}>
        <h2 ref={heading} tabIndex={-1} className={styles.title}>
          {title}
        </h2>
        <IconButton icon={iconName.close} label={t("panel.close")} onPress={onClose} />
      </div>
      <div className={styles.body}>{children}</div>
    </aside>
  );
}

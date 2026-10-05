import { t } from "@bugaboo/messages";
import { Button, cx, Icon, IconButton, iconName, useLayout } from "@bugaboo/ui";
import { useEffect, useRef, type ReactNode } from "react";

import styles from "./SidePanel.module.css";
import { useEscape } from "./use-escape.ts";

export interface SidePanelProps {
  /** What the panel shows: the name of the item that is open. */
  readonly title: string;
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/**
 * The panel at the side of a list (UX §3.4, §5.1): a simple item opens here and the list stays in
 * view. It takes the focus when it opens, closes with Esc or its button, and gives the focus back
 * to what had it before. A phone shows it as a screen of its own, with *Back*.
 */
export function SidePanel({ title, onClose, children }: SidePanelProps) {
  const heading = useRef<HTMLHeadingElement>(null);
  const phone = useLayout() === "phone";
  useEffect(() => {
    const before = document.activeElement;
    heading.current?.focus();
    return () => {
      if (before instanceof HTMLElement) {
        before.focus();
      }
    };
  }, []);
  const panel = useEscape<HTMLElement>(onClose);
  return (
    <aside ref={panel} className={cx(styles.panel, phone && styles.screen)} aria-label={title}>
      <div className={styles.head}>
        {phone ? (
          <Button quiet onPress={onClose}>
            <Icon name={iconName.back} />
            {t("shell.back")}
          </Button>
        ) : null}
        <h2 ref={heading} tabIndex={-1} className={styles.title}>
          {title}
        </h2>
        {phone ? null : (
          <IconButton icon={iconName.close} label={t("panel.close")} onPress={onClose} />
        )}
      </div>
      <div className={styles.body}>{children}</div>
    </aside>
  );
}

import { cx } from "@bugaboo/ui";

import styles from "./ThemePreview.module.css";

export interface ThemePreviewProps {
  /** The theme to show, by its ID on the page (`data-preview-look`). */
  readonly theme: string;
}

/**
 * A small sample of a theme, drawn in that theme's own colors, so it can be chosen by sight.
 * It says nothing to a screen reader: the name of the theme next to it does.
 */
export function ThemePreview({ theme }: ThemePreviewProps) {
  return (
    <span className={cx(styles.sample)} data-preview-look={theme} aria-hidden>
      <span className={styles.bar} />
      <span className={styles.card}>
        <span className={styles.line} />
        <span className={cx(styles.line, styles.short)} />
      </span>
    </span>
  );
}

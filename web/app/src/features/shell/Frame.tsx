import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";

import { usePageFocus } from "../../shared/focus.ts";
import { CONTENT_HREF, CONTENT_ID } from "./content-id.ts";
import styles from "./Shell.module.css";
import { ThemeBar } from "./ThemeBar.tsx";
import { useAppearance } from "@bugaboo/ui";

export interface FrameProps {
  /** The top of the page: the name of the product. */
  readonly header: ReactNode;
  readonly children: ReactNode;
}

/**
 * The frame of the pages before sign-in: skip link, header, the content, and the choice of theme.
 * (The signed-in app has its own frame, `AppLayout`.) When the person goes to another page, the
 * focus moves to its heading.
 */
export function Frame({ header, children }: FrameProps) {
  usePageFocus(CONTENT_ID);
  useAppearance();
  return (
    <div className={styles.page}>
      <a className={styles.skip} href={CONTENT_HREF}>
        {t("shell.skip")}
      </a>
      <header className={styles.header}>{header}</header>
      <main id={CONTENT_ID} className={styles.main} tabIndex={-1}>
        {children}
      </main>
      <footer className={styles.footer}>
        <ThemeBar />
      </footer>
    </div>
  );
}

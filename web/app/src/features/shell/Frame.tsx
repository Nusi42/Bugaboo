import { t } from "@bugaboo/messages";
import { useLocation } from "@tanstack/react-router";
import { useEffect, useRef, type ReactNode } from "react";

import styles from "./Shell.module.css";
import { ThemeBar } from "./ThemeBar.tsx";

/** The id the "skip to the content" link jumps to. */
const CONTENT_ID = "content";
const CONTENT_HREF = `#${CONTENT_ID}`;

export interface FrameProps {
  /** The top of the page: the name of the product, and a menu when somebody is signed in. */
  readonly header: ReactNode;
  readonly children: ReactNode;
}

/**
 * The frame of every page: skip link, header, the content, and the choice of theme. When the
 * person goes to another page, the focus moves to its heading, so a screen reader reads the new
 * page instead of staying on the link that was pressed.
 */
export function Frame({ header, children }: FrameProps) {
  const { pathname } = useLocation();
  const shownPage = useRef(pathname);
  useEffect(() => {
    if (shownPage.current !== pathname) {
      shownPage.current = pathname;
      const content = document.querySelector<HTMLElement>(`#${CONTENT_ID}`);
      (content?.querySelector<HTMLElement>("h1") ?? content)?.focus();
    }
  }, [pathname]);
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

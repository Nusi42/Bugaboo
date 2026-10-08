import { Alert, alertTone, type LayoutMode } from "@bugaboo/ui";
import { Outlet } from "@tanstack/react-router";

import { messageOf } from "../../shared/errors.ts";
import { usePageFocus } from "../../shared/focus.ts";
import styles from "./AppLayout.module.css";
import { BottomBar } from "./BottomBar.tsx";
import { CONTENT_ID, scrollRestoration } from "./content-id.ts";
import { TopBar } from "./TopBar.tsx";

export interface ShellColumnProps {
  readonly layout: LayoutMode;
  /** Why signing out did not work, if it did not. */
  readonly signOutProblem: unknown;
}

/** Everything next to the sidebar: the top bar, the page, and on a phone the bar at the bottom. */
export function ShellColumn({ layout, signOutProblem }: ShellColumnProps) {
  usePageFocus(CONTENT_ID);
  const phone = layout === "phone";
  return (
    <div className={styles.column}>
      <TopBar layout={layout} />
      <main id={CONTENT_ID} className={styles.main} tabIndex={-1} {...scrollRestoration}>
        <div className={styles.inner}>
          {signOutProblem === null ? null : (
            <Alert tone={alertTone.danger}>{messageOf(signOutProblem)}</Alert>
          )}
          <Outlet />
        </div>
      </main>
      {phone ? <BottomBar /> : null}
    </div>
  );
}

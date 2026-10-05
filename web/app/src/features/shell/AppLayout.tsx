import { t } from "@bugaboo/messages";
import { useAppearance, useLayout } from "@bugaboo/ui";

import styles from "./AppLayout.module.css";
import { CONTENT_HREF } from "./content-id.ts";
import { ShellColumn } from "./ShellColumn.tsx";
import { ShellOverlays } from "./ShellOverlays.tsx";
import { Sidebar } from "./Sidebar.tsx";
import { useSidebarNarrow } from "./sidebar-state.ts";
import { useShell } from "./use-shell.ts";

/**
 * The frame around the pages of somebody who is signed in (UX §3): on a computer a sidebar and a
 * top bar, on a tablet a menu that slides in, on a phone one screen at a time with a bar of
 * labelled buttons at the bottom. The same pages, the same shortcuts and the same themes in all.
 */
export function AppLayout() {
  const layout = useLayout();
  const { narrow } = useSidebarNarrow();
  const { person, signOutProblem } = useShell();
  useAppearance();
  const computer = layout === "computer";
  return (
    <div className={styles.shell} data-layout={layout}>
      <a className={styles.skip} href={CONTENT_HREF}>
        {t("shell.skip")}
      </a>
      {computer ? <Sidebar narrow={narrow} {...person} /> : null}
      <ShellColumn layout={layout} signOutProblem={signOutProblem} />
      <ShellOverlays layout={layout} {...person} />
    </div>
  );
}

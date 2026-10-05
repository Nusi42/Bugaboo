import { t } from "@bugaboo/messages";

import { useSidebarSections } from "./sidebar-sections.ts";
import styles from "./Sidebar.module.css";

/** The parts features add to the sidebar, such as the project tree. */
export function Sections({
  narrow,
  onNavigate,
}: {
  readonly narrow: boolean;
  readonly onNavigate?: (() => void) | undefined;
}) {
  const sections = useSidebarSections();
  return sections.map((section) => (
    <section key={section.heading} className={styles.section} aria-label={t(section.heading)}>
      {narrow ? null : <h2 className={styles.heading}>{t(section.heading)}</h2>}
      {section.render({ narrow, ...(onNavigate === undefined ? {} : { onNavigate }) })}
    </section>
  ));
}

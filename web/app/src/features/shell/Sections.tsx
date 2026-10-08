import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useSidebarSections } from "./sidebar-sections.ts";

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
    <section key={section.heading} {...partOf(part.sidebarSection)} aria-label={t(section.heading)}>
      {narrow ? null : <h2 {...partOf(part.sectionHeading)}>{t(section.heading)}</h2>}
      {section.render({ narrow, ...(onNavigate === undefined ? {} : { onNavigate }) })}
    </section>
  ));
}

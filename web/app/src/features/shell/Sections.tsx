import { Fragment } from "react";

import { useSidebarSections } from "./sidebar-sections.ts";

/** The parts features add to the sidebar, such as the project tree, each with its own frame. */
export function Sections({
  narrow,
  onNavigate,
}: {
  readonly narrow: boolean;
  readonly onNavigate?: (() => void) | undefined;
}) {
  return useSidebarSections().map((section) => (
    <Fragment key={section.heading}>
      {section.render({
        narrow,
        place: "sidebar",
        ...(onNavigate === undefined ? {} : { onNavigate }),
      })}
    </Fragment>
  ));
}

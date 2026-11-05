import type { MessageKey } from "@bugaboo/messages";

import { addQuickSearchSource, addSidebarSection, SectionFrame } from "../shell/index.ts";
import { ProjectTree } from "./ProjectTree.tsx";
import { projectsSource } from "./quick-search.ts";

/** The heading of the project tree. */
const heading = "projects.heading" satisfies MessageKey;

let registered = false;

/**
 * Puts the projects where the shell has room for them (WP-03's extension points): the project tree
 * in the sidebar and in the *More* sheet of a phone, and projects among the sources of quick
 * search. Called once when the app starts.
 */
export function registerProjects(): void {
  if (registered) {
    return;
  }
  registered = true;
  addSidebarSection({
    heading,
    render: (view) => (
      <SectionFrame heading={heading} view={view}>
        <ProjectTree
          narrow={view.narrow}
          {...(view.onNavigate === undefined ? {} : { onNavigate: view.onNavigate })}
        />
      </SectionFrame>
    ),
  });
  addQuickSearchSource(projectsSource);
}

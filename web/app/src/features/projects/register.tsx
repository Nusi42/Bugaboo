import { addQuickSearchSource, addSidebarSection } from "../shell/index.ts";
import { ProjectTree } from "./ProjectTree.tsx";
import { projectsSource } from "./quick-search.ts";

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
    heading: "projects.heading",
    render: ({ narrow, onNavigate }) => (
      <ProjectTree narrow={narrow} {...(onNavigate === undefined ? {} : { onNavigate })} />
    ),
  });
  addQuickSearchSource(projectsSource);
}

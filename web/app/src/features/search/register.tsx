import { addSidebarSection } from "../shell/index.ts";
import { SavedSearchesSection } from "./SavedSearchesSection.tsx";

let registered = false;

/**
 * Puts *Saved searches* where the shell has room for them (the extension point of the sidebar):
 * the page of *Search* and the saved searches, in the sidebar and in the *More* sheet of a phone.
 * Called once when the app starts.
 */
export function registerSearch(): void {
  if (registered) {
    return;
  }
  registered = true;
  addSidebarSection({
    heading: "search.saved-heading",
    render: ({ narrow, onNavigate }) => (
      <SavedSearchesSection narrow={narrow} {...(onNavigate === undefined ? {} : { onNavigate })} />
    ),
  });
}

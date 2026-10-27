import { issuePanelAddress } from "../../src/shared/search-address.ts";

export { issueAddress } from "../../src/shared/paths.ts";

/** The address of the list of a project with one issue open in the side panel, as the app writes it. */
export function panelAddress(project: string, key: string): string {
  return issuePanelAddress(undefined, project, key);
}

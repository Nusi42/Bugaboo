import { partOfAddress, paths, routeParam } from "./paths.ts";
import { readSearch } from "./search-address.ts";

/** The start of every address of a project, like `/p/`. */
const PROJECT_PREFIX = paths.project.slice(0, paths.project.indexOf("$"));

/** The project of an address under a project (`/p/WEB/settings`), if it is one. */
export function projectInAddress(pathname: string): string | undefined {
  if (!pathname.startsWith(PROJECT_PREFIX)) {
    return undefined;
  }
  const word = pathname.slice(PROJECT_PREFIX.length).split("/")[0];
  return word === undefined || word === "" ? undefined : decodeURIComponent(word);
}

/** The short name of the project an issue ID starts with (`WEB` of `WEB-142`). */
function projectOfIssue(issue: string): string | undefined {
  const dash = issue.lastIndexOf("-");
  return dash > 0 ? issue.slice(0, dash) : undefined;
}

/**
 * The project of the place, which says whether AI counts there: the project of a list, a page of a
 * project, or the project an issue ID starts with. None for a page that belongs to no project.
 */
export function projectOf(pathname: string, search: object): string | undefined {
  const issue = partOfAddress(paths.issue, routeParam.issue, pathname);
  if (issue !== undefined) {
    return projectOfIssue(issue);
  }
  return readSearch({ pathname, search }).project ?? projectInAddress(pathname);
}

import { Navigate } from "@tanstack/react-router";

import { NotFoundPage } from "../shell/index.ts";
import { tabAddress, tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** For a person who may do neither of the two: the first other tab they may open, else nothing. */
export function OtherTab({ project }: { readonly project: OneProject }) {
  const [first] = tabsOf(project);
  return first === undefined ? (
    <NotFoundPage />
  ) : (
    <Navigate to={tabAddress(first, project.project.short_name)} replace />
  );
}

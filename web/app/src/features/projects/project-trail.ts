import { useMemo } from "react";

import { projectAddress } from "../../shared/paths.ts";
import { usePageTrail, type TrailStep } from "../shell/index.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * *Where you are* on a page of a project: the projects above it that the person sees, the project
 * itself, and `last` for a page of the project such as its settings. Nothing is said about a
 * parent the person cannot see: the server leaves it out of the answer.
 */
export function useProjectTrail(project: OneProject | undefined, last?: TrailStep): void {
  const steps = useMemo((): readonly TrailStep[] | undefined => {
    if (project === undefined) {
      return undefined;
    }
    const above = project.trail.map((step) => ({
      label: step.name,
      href: projectAddress(step.short_name),
    }));
    const here = {
      label: project.project.name,
      href: projectAddress(project.project.short_name),
    };
    return last === undefined ? [...above, here] : [...above, here, last];
  }, [project, last]);
  usePageTrail(steps);
}

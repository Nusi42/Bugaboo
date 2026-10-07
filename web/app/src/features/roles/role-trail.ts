import { t } from "@bugaboo/messages";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTrail, type TrailStep } from "../shell/index.ts";

/** The trail of *where you are* on the page of a role: *Roles & permissions*, then the role. */
export function useRoleTrail(name: string | undefined): void {
  const steps = useMemo((): readonly TrailStep[] | undefined => {
    if (name === undefined) {
      return undefined;
    }
    return [
      { label: t("nav.organization-settings"), href: paths.organizationSettings },
      { label: t("nav.roles"), href: paths.roles },
      { label: name, href: paths.roles },
    ];
  }, [name]);
  usePageTrail(steps);
}

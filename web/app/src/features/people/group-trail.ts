import { t } from "@bugaboo/messages";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTrail, type TrailStep } from "../shell/index.ts";

/** The trail of *where you are* on the screen of one group on a phone: *Groups*, then the group; Back leads to the list. */
export function useGroupTrail(name: string | undefined): void {
  const steps = useMemo((): readonly TrailStep[] | undefined => {
    if (name === undefined) {
      return undefined;
    }
    return [
      { label: t("nav.organization-settings"), href: paths.organizationSettings },
      { label: t("nav.groups"), href: paths.groups },
      { label: name, href: paths.groups },
    ];
  }, [name]);
  usePageTrail(steps);
}

import { t } from "@bugaboo/messages";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTrail, type TrailStep } from "../shell/index.ts";

/** The trail of *where you are* on the page of a person: *People*, then the person. */
export function usePersonTrail(name: string | undefined): void {
  const steps = useMemo((): readonly TrailStep[] | undefined => {
    if (name === undefined) {
      return undefined;
    }
    return [
      { label: t("nav.organization-settings"), href: paths.organizationSettings },
      { label: t("nav.people"), href: paths.people },
      { label: name, href: paths.people },
    ];
  }, [name]);
  usePageTrail(steps);
}

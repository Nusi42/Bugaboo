import { t } from "@bugaboo/messages";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTrail } from "../shell/index.ts";

/** *Timesheet › Working hours of others*, in the bar at the top of the page (a computer and a phone). */
export function useOthersTrail(): void {
  const trail = useMemo(
    () => [
      { label: t("nav.timesheet"), href: paths.time },
      { label: t("time.others.title"), href: paths.timeOthers },
    ],
    [],
  );
  usePageTrail(trail);
}

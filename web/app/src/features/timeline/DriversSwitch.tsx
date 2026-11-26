import { t } from "@bugaboo/messages";
import { Switch } from "@bugaboo/ui";

import type { ListView } from "../../shared/list-view.ts";

export interface DriversSwitchProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
}

/**
 * The switch *Deadline drivers* (PLAN-05), part of the view: on a computer it is the Legend's item
 * of the outline, beside the swatch it switches on; on a phone, which has no Legend, it stands in
 * the tools.
 */
export function DriversSwitch({ view, onView }: DriversSwitchProps) {
  const { timeline } = view;
  return (
    <Switch
      label={t("timeline.deadline-drivers")}
      isSelected={timeline.deadline_drivers ?? false}
      onChange={(on) => {
        onView({ ...view, timeline: { ...timeline, deadline_drivers: on } });
      }}
    />
  );
}

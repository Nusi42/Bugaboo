import type { ReactNode } from "react";

import type { RowGroup } from "./details-order.ts";
import { detailFields as fields } from "./details-order.ts";
import { FoundInReleaseRow, PlannedForReleaseRow } from "./ReleaseRows.tsx";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";

/** The two rows that name releases, each where the layout places its field: *Planned for release*, *Found in release*. */
export function releaseGroups({
  blocks,
  changes,
}: ChangeableRowsProps): readonly RowGroup<ReactNode>[] {
  return [
    {
      places: [fields.plannedForRelease],
      node: (
        <PlannedForReleaseRow key={fields.plannedForRelease} blocks={blocks} changes={changes} />
      ),
    },
    {
      places: [fields.foundInRelease],
      node: <FoundInReleaseRow key={fields.foundInRelease} blocks={blocks} changes={changes} />,
    },
  ];
}

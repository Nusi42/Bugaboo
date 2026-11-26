import { gridColumns } from "@bugaboo/api-client";

import { filled, paths, routeParam } from "../../src/shared/paths.ts";
import { viewports } from "../../../tooling/viewports.ts";
import { signInThroughApi } from "../support/accounts.ts";
import { readyMadeBox } from "../support/dashboard-issues.ts";
import { createDashboard, type AskedBox } from "../support/dashboards.ts";
import { sampleAdministrator } from "../support/sample.ts";
import { type Instance } from "../support/server.ts";

// What the pictures of the views are taken of beside the fixed search: the size of each device and the dashboard
// of four ready-made boxes (architecture §11, *Visual tests*).

/** The size of each device of the settings. */
const sizes: Readonly<Record<string, { readonly width: number; readonly height: number }>> = {
  computer: viewports.computer,
  phone: viewports.phoneOfVisualTests,
};

/** The size of a device of the settings; a device without one stops the suite with what to add. */
export function sizeOf(device: string): { readonly width: number; readonly height: number } {
  const size = sizes[device];
  if (size === undefined) {
    throw new RangeError(`The device ${device} of the settings has no size in scene.ts.`);
  }
  return size;
}

/** The four ready-made boxes of the dashboard that is pictured: two rows of two on a computer. */
const boxes = [
  "open-bugs-by-priority",
  "issues-by-status-group",
  "my-open-issues",
  "time-on-this-search",
] as const;

/** The address of the dashboard of the pictures, once it is made. */
export let dashboard = "";

/**
 * The boxes side by side in the order given, as many to a row as fit the columns of the grid, each row as high as its
 * highest box, so no box stands on another.
 */
function placed(asked: readonly AskedBox[]): AskedBox[] {
  let column = 0;
  let row = 0;
  let rowHeight = 0;
  return asked.map((box) => {
    if (column + box.width > gridColumns) {
      row += rowHeight;
      column = 0;
      rowHeight = 0;
    }
    const at = { ...box, column, row };
    column += box.width;
    rowHeight = Math.max(rowHeight, box.height);
    return at;
  });
}

/** Creates the dashboard with the four ready-made boxes, as its administrator. */
export async function createDashboardOfPictures(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, sampleAdministrator("fixed"));
  try {
    const asked = await Promise.all(boxes.map((which) => readyMadeBox(admin, which)));
    const made = await createDashboard(admin, "Pictures", placed(asked));
    dashboard = filled(paths.dashboard, routeParam.dashboard, made.id);
  } finally {
    await admin.dispose();
  }
}

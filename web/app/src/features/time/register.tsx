import { addOverlayHost, addTimePart } from "../shell/index.ts";
import { TimeAsks } from "./TimeAsks.tsx";
import { TimePart } from "./TimePart.tsx";

let registered = false;

/**
 * Puts the timer where the shell has room for it: the part at the bottom of the sidebar, at the top
 * of *Home* and in the bar of a phone, and the windows and questions of the timer over any page.
 * Called once when the app starts.
 */
export function registerTime(): void {
  if (registered) {
    return;
  }
  registered = true;
  addTimePart({
    id: "time",
    render: (view) => <TimePart {...view} />,
  });
  addOverlayHost({ id: "time-asks", render: () => <TimeAsks /> });
}

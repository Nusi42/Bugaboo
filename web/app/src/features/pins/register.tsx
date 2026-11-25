import { addSidebarSection } from "../shell/index.ts";
import { PinnedSection } from "./PinnedSection.tsx";
import { pinsHeading } from "./pins.ts";

let registered = false;

/**
 * Puts *Pinned* into the sidebar and the sheet *More* (CFG-05). The pins join before the project tree, so *Pinned*
 * stands above it. Called once when the app starts.
 */
export function registerPins(): void {
  if (registered) {
    return;
  }
  registered = true;
  addSidebarSection({
    heading: pinsHeading,
    render: (view) => <PinnedSection view={view} />,
  });
}

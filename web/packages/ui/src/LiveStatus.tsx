import type { ReactNode } from "react";

import { live } from "./live.ts";

/**
 * A part on the screen whose changes a screen reader says when the person pauses, without the focus moving to it
 * (WCAG 4.1.3): the state of something that goes on in the background, such as a file being made. Unlike `Announce`,
 * what it holds is seen.
 */
export function LiveStatus({ children }: { readonly children: ReactNode }) {
  return <div role={live.status}>{children}</div>;
}

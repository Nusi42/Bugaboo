import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

const HistoryBlock = lazyRouteComponent(() => import("./HistoryBlock.tsx"), "HistoryBlock");

/** Puts *History* into its slot at the end of the main column of the issue page (WP-07). */
export function registerHistoryBlock(): void {
  addIssueBlock(issueSlots.history, { id: "history", order: 0, component: HistoryBlock });
}

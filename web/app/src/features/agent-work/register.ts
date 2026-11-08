import { t } from "@bugaboo/messages";
import { lazy, Suspense, createElement, type ReactNode } from "react";

import {
  addIssueBlock,
  addMoreEntry,
  addSelectionBar,
  issueSlots,
  type MoreDialogProps,
  type SelectionBarProps,
} from "../issues/index.ts";

// Loaded when an issue is first opened or issues are first ticked, so the first page stays small
// (coding-standards section 6).
const LazyBlock = lazy(async () => {
  const loaded = await import("./AgentWorkBlock.tsx");
  return { default: loaded.AgentWorkBlock };
});
const LazyWorkedOnBy = lazy(async () => {
  const loaded = await import("./WorkedOnByRow.tsx");
  return { default: loaded.WorkedOnByRow };
});
const LazyBar = lazy(async () => {
  const loaded = await import("./HandOverBar.tsx");
  return { default: loaded.HandOverBar };
});
const LazyEntry = lazy(async () => {
  const loaded = await import("./HandOverEntry.tsx");
  return { default: loaded.HandOverEntry };
});

/** Puts the agent work, *worked on by*, the bar of ticked issues and the entry of *More* where they belong (WP-11). */
export function registerAgentWork(): void {
  addIssueBlock(issueSlots.agentWork, {
    id: "agent-work",
    order: 0,
    component: (props) =>
      createElement(Suspense, { fallback: null }, createElement(LazyBlock, props)),
  });
  addIssueBlock(issueSlots.details, {
    id: "agent-worked-on-by",
    order: 100,
    // Right after *Assigned to*, which the agent never is: the issue keeps its person.
    places: ["assigned-to"],
    component: (props) =>
      createElement(Suspense, { fallback: null }, createElement(LazyWorkedOnBy, props)),
  });
  addSelectionBar({
    id: "hand-over",
    order: 10,
    component: (props: SelectionBarProps): ReactNode =>
      createElement(Suspense, { fallback: null }, createElement(LazyBar, props)),
  });
  addMoreEntry({
    id: "hand-over",
    order: 0,
    label: () => t("agent-work.hand-over"),
    dialog: (props: MoreDialogProps) =>
      createElement(Suspense, { fallback: null }, createElement(LazyEntry, props)),
  });
}

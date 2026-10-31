import { useAppearance } from "@bugaboo/ui";

/**
 * Whether the list is *Compact*: it follows the person's *Spacing* in *My settings › Appearance*,
 * as every other screen does (ADR-0034, amendment), so a list or a saved search has none of its own.
 */
export function useCompact(): boolean {
  return useAppearance().appearance.spacing === "compact";
}

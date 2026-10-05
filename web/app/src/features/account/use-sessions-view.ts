import { useQuery } from "@tanstack/react-query";

import { useAfterDelay } from "../../shared/use-optimistic.ts";
import { sessionsOptions } from "./sessions.ts";

/**
 * The list of places, and whether grey shapes should stand in for it: only when it takes longer
 * than 0.3 s to arrive, so a quick answer never flashes them (UX §6.1).
 */
export function useSessionsView() {
  const sessions = useQuery(sessionsOptions());
  return { sessions, shapes: useAfterDelay(sessions.isPending) };
}

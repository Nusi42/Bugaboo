import { lazy, Suspense } from "react";

import { useTimeAsk } from "./time-asks.ts";

/** Each window loads when it first opens, so the first page carries none of them. */
const StopTimerAsk = lazy(async () => ({
  default: (await import("./StopTimerAsk.tsx")).StopTimerAsk,
}));
const SwitchIssueWindow = lazy(async () => ({
  default: (await import("./SwitchIssueWindow.tsx")).SwitchIssueWindow,
}));
const ChangeStartWindow = lazy(async () => ({
  default: (await import("./ChangeStartWindow.tsx")).ChangeStartWindow,
}));
const ThrowAwayAsk = lazy(async () => ({
  default: (await import("./ThrowAwayAsk.tsx")).ThrowAwayAsk,
}));

/**
 * The windows and questions of the timer, over any page: *Stop timer*, *Switch issue*, *Change start
 * time* and *Throw away*. What is asked is set by `askTime`; one at a time.
 */
export function TimeAsks() {
  const ask = useTimeAsk();
  if (ask === null) {
    return null;
  }
  return (
    <Suspense fallback={null}>
      {ask.kind === "stop" ? <StopTimerAsk switchingTo={ask.switchingTo} /> : null}
      {ask.kind === "switch" ? <SwitchIssueWindow /> : null}
      {ask.kind === "start-time" ? <ChangeStartWindow /> : null}
      {ask.kind === "throw-away" ? <ThrowAwayAsk /> : null}
    </Suspense>
  );
}

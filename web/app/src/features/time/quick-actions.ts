import { t } from "@bugaboo/messages";

import { messageOf } from "../../shared/errors.ts";
import { queryClient } from "../../shared/query-client.ts";
import type { QuickAction } from "../shell/index.ts";
import { openIssueId } from "./open-issue-id.ts";
import { askTime } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { startTimer } from "./timer-start.ts";
import { timerQuery } from "./use-timer.ts";

/**
 * *Start timer* and *Stop timer* in *Quick search* (CFG-08): the first starts the timer on the open
 * issue, or asks which issue when none is open; the second opens *Stop timer*, and does nothing while
 * no timer runs.
 */
export const timerQuickActions: readonly QuickAction[] = [
  {
    id: "start-timer",
    title: t("time.timer.start"),
    run: () => {
      const open = openIssueId(window.location.pathname);
      if (open === undefined) {
        askTime({ kind: "switch" });
      } else {
        void startTimer(open);
      }
    },
  },
  {
    id: "stop-timer",
    title: t("time.timer.stop-timer"),
    run: () => {
      queryClient.query({ ...timerQuery, staleTime: 0 }).then(
        (reading) => {
          if (reading.timer.running !== undefined && reading.timer.running !== null) {
            askTime({ kind: "stop" });
          }
        },
        (error: unknown) => {
          tellTime(messageOf(error), "danger");
        },
      );
    },
  },
];

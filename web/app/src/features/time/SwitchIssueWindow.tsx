import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { IssuePicker } from "../search/index.ts";
import { EntryWindow } from "./EntryWindow.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { startTimer } from "./timer-start.ts";
import { useTimer } from "./use-timer.ts";

/**
 * *Switch issue* (TIME-01): the time so far is saved on the issue the timer runs on, with its
 * defaults, and the timer goes on with the issue that is picked here (by its ID or words of its title).
 */
export function SwitchIssueWindow() {
  const running = useTimer().data?.timer.running ?? undefined;
  return (
    <EntryWindow
      title={t("time.switch.title")}
      isOpen
      onOpenChange={closeTimeAsk}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={closeTimeAsk}>
            {t("common.cancel")}
          </Button>
        </div>
      }
    >
      <div {...partOf(part.stack)}>
        {running?.readable_id === undefined || running.readable_id === null ? null : (
          <p {...partOf(part.muted)}>{t("time.switch.help", { issue: running.readable_id })}</p>
        )}
        <IssuePicker
          onPick={(key) => {
            closeTimeAsk();
            void startTimer(key);
          }}
        />
      </div>
    </EntryWindow>
  );
}

import { t } from "@bugaboo/messages";
import { Alert, alertTone, IconButton, iconName, type AlertTone } from "@bugaboo/ui";
import { useEffect } from "react";

import { IdsIn } from "./IdsIn.tsx";
import { forgetNotice, useTimeNotice, type NoticeTone } from "./time-notice.ts";

/** How long a notice stays: long enough to read a sentence twice. */
const NOTICE_MS = 12_000;

const tones: Readonly<Record<NoticeTone, AlertTone>> = {
  success: alertTone.success,
  info: alertTone.info,
  danger: alertTone.danger,
};

/** What the timer tells after it saved, threw away or could not do something: one sentence that goes away by itself. */
export function TimerNotice() {
  const notice = useTimeNotice();
  useEffect(() => {
    const timer = notice === null ? undefined : setTimeout(forgetNotice, NOTICE_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [notice]);
  if (notice === null) {
    return null;
  }
  return (
    <Alert tone={tones[notice.tone]}>
      <span>
        <IdsIn sentence={notice.text} ids={notice.ids} lengths={notice.lengths} />
      </span>
      <IconButton icon={iconName.close} label={t("common.close")} onPress={forgetNotice} />
    </Alert>
  );
}

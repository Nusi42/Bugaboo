import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import { lazy, Suspense, useState } from "react";

import { WaitingWindow } from "../../shared/WaitingWindow.tsx";
import { windowForms } from "../../shared/window-forms.ts";
import type { IssueView } from "./slots.ts";
import { shownPercent, timeFactorTitle, useTimeFactor, type TimeFactor } from "./time-factor.ts";

/** The window loads when it first opens, so the page that offers *Change* does not carry the form. */
const TimeFactorWindow = lazy(async () => ({
  default: (await import("./TimeFactorWindow.tsx")).TimeFactorWindow,
}));

export interface TimeFactorChangeProps {
  /**
   * The factor the row beside it already shows, when that is ahead of the answer of the server (after
   * a change of the people on the issue), so *Change* is offered as the row says; otherwise the
   * answer.
   */
  readonly factor?: TimeFactor;
  /** The issue: its *Estimated time*, when the reader may read it. */
  readonly issue: Pick<IssueView, "estimated_time">;
  readonly readableId: string;
  /** Whether the reader may read *Estimated time*: only then can the factor be given as a length. */
  readonly mayReadEstimate: boolean;
  /** The window closed, saved or not: the page reads what depends on the factor again. */
  readonly onClosed?: () => void;
}

/**
 * *Change* of the time factor (PLAN-14, UX §4.4 and §4.7), which opens the window *How long will it
 * take you?*: on the issue page and on the card of a bar. Whether it is offered the server says
 * (`may_change`), by the rule that also decides a save.
 */
export function TimeFactorChange({
  factor: shown,
  issue,
  readableId,
  mayReadEstimate,
  onClosed,
}: TimeFactorChangeProps) {
  const read = useTimeFactor(readableId).data;
  const factor = shown ?? read;
  const [open, setOpen] = useState(false);
  const percent = factor === undefined ? undefined : shownPercent(factor);
  const close = (): void => {
    setOpen(false);
    onClosed?.();
  };
  if (factor === undefined || percent === undefined || !factor.may_change) {
    return null;
  }
  return (
    <>
      <Button
        look={buttonLook.quiet}
        aria-label={t("issues.time-factor-change-name")}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("issues.time-factor-change")}
      </Button>
      {open ? (
        <Suspense
          fallback={
            <WaitingWindow
              form={windowForms.timeFactor}
              title={timeFactorTitle(factor)}
              onClose={close}
            />
          }
        >
          <TimeFactorWindow
            issue={issue}
            readableId={readableId}
            factor={factor}
            percent={percent}
            mayReadEstimate={mayReadEstimate}
            onClose={close}
          />
        </Suspense>
      ) : null}
    </>
  );
}

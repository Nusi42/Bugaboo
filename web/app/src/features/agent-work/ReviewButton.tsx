import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import type { Work } from "./agent-work.ts";
import { ReviewDialog } from "./ReviewDialog.tsx";

/** *Review*: opens the window of the review of a result that is ready (UX §4.17). */
export function ReviewButton({
  work,
  issue,
  statusChanges,
}: {
  readonly work: Work;
  readonly issue: string;
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
}) {
  const [open, setOpen] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  return (
    <>
      <Button
        look={buttonLook.primary}
        onPress={() => {
          setProblem(null);
          setOpen(true);
        }}
      >
        {t("agent-work.review")}
      </Button>
      {open ? (
        <ReviewDialog
          work={work}
          issue={issue}
          statusChanges={statusChanges}
          isOpen
          onOpenChange={setOpen}
          onFailed={(error) => {
            setProblem(messageOf(error));
          }}
        />
      ) : null}
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
    </>
  );
}

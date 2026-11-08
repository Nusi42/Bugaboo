import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import type { SelectionBarProps } from "../issues/index.ts";
import { HandOverDialog } from "./HandOverDialog.tsx";

/**
 * *Hand over to an agent* among the actions for ticked issues (AI-31, UX §4.17): one window for all
 * of them, and a sentence under the list that says how many were handed over.
 */
export function HandOverBar({ selection }: SelectionBarProps) {
  const [open, setOpen] = useState(false);
  const [handed, setHanded] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  if (selection.ids.length === 0 && handed === null && problem === null) {
    return null;
  }
  return (
    <>
      {selection.ids.length === 0 ? null : (
        <section {...partOf(part.manyChangeBar)} aria-label={t("agent-work.hand-over")}>
          <Button
            onPress={() => {
              setHanded(null);
              setProblem(null);
              setOpen(true);
            }}
          >
            {t("agent-work.hand-over")}
          </Button>
        </section>
      )}
      {open ? (
        <HandOverDialog
          issues={selection.ids}
          isOpen
          onOpenChange={setOpen}
          onFailed={(error) => {
            setProblem(messageOf(error));
          }}
          onHandedOver={() => {
            setHanded(selection.ids.length);
            selection.clear();
          }}
        />
      ) : null}
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
      {handed === null ? null : (
        <output>
          <Alert tone={alertTone.success}>{t("agent-work.handed", { count: handed })}</Alert>
        </output>
      )}
    </>
  );
}

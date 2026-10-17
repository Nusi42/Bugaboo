import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import type { SelectionBarProps } from "../issues/index.ts";
import { actionName } from "./action-names.ts";
import { afterApply } from "./after-apply.ts";
import { manyChangeKind, type ManyChangeApplied, type ManyChangeKind } from "./many-changes.ts";
import { ManyChangeDialog } from "./ManyChangeDialog.tsx";
import { ManyChangeOutcome } from "./ManyChangeOutcome.tsx";

/** The five things, in the order of the buttons. */
const kinds: readonly ManyChangeKind[] = [
  manyChangeKind.change,
  manyChangeKind.move,
  manyChangeKind.label,
  manyChangeKind.relation,
  manyChangeKind.delete,
];

/**
 * The bar under the list while issues are ticked (ISS-09): the five things that can be done to
 * them at once. Each opens a dialog that shows what would change before anything does.
 */
export function ManyChangeBar({ selection, project }: SelectionBarProps) {
  const [open, setOpen] = useState<ManyChangeKind | null>(null);
  const [applied, setApplied] = useState<ManyChangeApplied | null>(null);
  return (
    <div {...partOf(part.stack)}>
      {selection.ids.length === 0 ? null : (
        <section {...partOf(part.manyChangeBar)} aria-label={t("many-changes.bar")}>
          {kinds.map((kind) => (
            <Button
              key={kind}
              onPress={() => {
                setApplied(null);
                setOpen(kind);
              }}
            >
              {actionName(kind)}
            </Button>
          ))}
        </section>
      )}
      {open === null ? null : (
        <ManyChangeDialog
          kind={open}
          project={project}
          issues={selection.ids}
          isOpen
          onOpenChange={(next) => {
            if (!next) {
              setOpen(null);
            }
          }}
          onApplied={(done) => {
            setApplied(done);
            afterApply(done, selection.clear);
          }}
        />
      )}
      {applied === null ? null : <ManyChangeOutcome applied={applied} />}
    </div>
  );
}

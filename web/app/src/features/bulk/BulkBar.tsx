import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import type { SelectionBarProps } from "../issues/index.ts";
import { actionName } from "./action-names.ts";
import { afterApply } from "./after-apply.ts";
import { bulkKind, type BulkKind } from "./bulk.ts";
import { BulkDialog } from "./BulkDialog.tsx";
import { BulkOutcome } from "./BulkOutcome.tsx";
import type { BulkApplied } from "./use-bulk-dialog.ts";

/** The five things, in the order of the buttons. */
const kinds: readonly BulkKind[] = [
  bulkKind.change,
  bulkKind.move,
  bulkKind.label,
  bulkKind.relation,
  bulkKind.delete,
];

/**
 * The bar under the list while issues are ticked (ISS-09): the five things that can be done to
 * them at once. Each opens a dialog that shows what would change before anything does.
 */
export function BulkBar({ selection }: SelectionBarProps) {
  const [open, setOpen] = useState<BulkKind | null>(null);
  const [applied, setApplied] = useState<BulkApplied | null>(null);
  return (
    <div {...partOf(part.stack)}>
      <section {...partOf(part.bulkBar)} aria-label={t("bulk.bar")}>
        {kinds.map((kind) => (
          <Button
            key={kind}
            onPress={() => {
              setOpen(kind);
            }}
          >
            {actionName(kind)}
          </Button>
        ))}
      </section>
      {open === null ? null : (
        <BulkDialog
          kind={open}
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
      {applied === null ? null : <BulkOutcome applied={applied} />}
    </div>
  );
}

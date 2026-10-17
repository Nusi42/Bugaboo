import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import type { Draft } from "./draft-model.ts";
import type { Move } from "./use-publish.ts";
import type { PublishPreview } from "./workflows.ts";

export interface PublishDialogProps {
  readonly preview: PublishPreview;
  readonly draft: Draft;
  readonly pending: boolean;
  readonly onConfirm: (moves: readonly Move[]) => void;
  readonly onCancel: () => void;
}

type Dropped = PublishPreview["dropped"][number];

/** A status an issue can go to, by its key and its name. */
interface Target {
  readonly value: string;
  readonly label: string;
}

/** The statuses an issue can go to: the ones the draft keeps and that have a key already. */
function targetsOf(draft: Draft): readonly Target[] {
  return draft.statuses.flatMap((status) =>
    status.key === null ? [] : [{ value: status.key, label: status.name }],
  );
}

/** One removed status: how many issues are in it, which of them the person sees, and where they go. */
function Where({
  dropped,
  targets,
  chosen,
  onChoose,
}: {
  readonly dropped: Dropped;
  readonly targets: readonly Target[];
  readonly chosen: string;
  readonly onChoose: (to: string) => void;
}) {
  const label = t("workflows.publish-where", {
    count: dropped.issues,
    status: dropped.status.name,
  });
  return (
    <div {...partOf(part.stack)}>
      <SelectField label={label} choices={targets} value={chosen} onChange={onChoose} />
      <ul {...partOf(part.list)}>
        {dropped.named.map((issue) => (
          <li key={issue.id}>{[issue.key, issue.title].filter(Boolean).join(" · ")}</li>
        ))}
      </ul>
      {dropped.unseen > 0 ? (
        <p {...partOf(part.muted)}>{t("workflows.publish-unseen", { count: dropped.unseen })}</p>
      ) : null}
      {dropped.blocked > 0 ? (
        <Alert tone={alertTone.danger}>
          {t("workflows.publish-blocked", { count: dropped.blocked, status: dropped.status.name })}
        </Alert>
      ) : null}
    </div>
  );
}

/**
 * The question of *Publish* (UX §5.4): the statuses that were removed while issues are still in
 * them, and where each group of issues goes. Issues the person cannot see are only counted. The
 * server judges every move again; this only collects the answers.
 */
export function PublishDialog(props: PublishDialogProps) {
  const { preview, draft, pending, onConfirm, onCancel } = props;
  const asked = preview.dropped.filter((each) => each.issues > 0);
  const targets = targetsOf(draft);
  const first = targets[0]?.value ?? "";
  const [chosen, setChosen] = useState<Readonly<Record<string, string>>>({});
  const total = asked.reduce((sum, each) => sum + each.issues, 0);
  const tooMany = total > preview.limit;
  const where = (each: Dropped): string => chosen[each.status.key] ?? first;
  const stuck = tooMany || asked.some((each) => each.blocked > 0) || pending || first === "";
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onCancel();
        }
      }}
      title={t("workflows.publish-title")}
      text={t("workflows.publish-text")}
      confirmLabel={t("workflows.publish-go", { count: total })}
      cancelLabel={t("common.cancel")}
      confirmDisabled={stuck}
      onConfirm={() => {
        onConfirm(asked.map((each) => ({ from: each.status.key, to: where(each) })));
      }}
    >
      {asked.map((each) => (
        <Where
          key={each.status.key}
          dropped={each}
          targets={targets}
          chosen={where(each)}
          onChoose={(next) => {
            setChosen({ ...chosen, [each.status.key]: next });
          }}
        />
      ))}
      {tooMany ? (
        <Alert tone={alertTone.danger}>
          {t("workflows.publish-too-many", { limit: preview.limit })}
        </Alert>
      ) : null}
    </AskDialog>
  );
}

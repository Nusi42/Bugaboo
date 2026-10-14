import { t } from "@bugaboo/messages";
import { Form, SubmitButton, partOf, part } from "@bugaboo/ui";

import { sortingKind } from "./change-preview.ts";
import { ShowChanges } from "./ChangePreview.tsx";
import { LabelFields } from "./LabelFields.tsx";
import type { Label, LabelGroup } from "./labels.ts";
import { SavedNote } from "./SavedNote.tsx";
import { useLabelDraft } from "./use-label-draft.ts";

/** The name, color and group of a label, saved on the version that was looked at. */
export function LabelForm({
  label,
  groups,
}: {
  readonly label: Label;
  readonly groups: readonly LabelGroup[];
}) {
  const draft = useLabelDraft(label);
  const renamed = draft.name.trim() !== "" && draft.name.trim() !== label.name;
  return (
    <>
      <Form onSubmit={draft.save}>
        <LabelFields
          label={label}
          groups={groups}
          name={draft.name}
          onName={draft.setName}
          color={draft.color}
          onColor={draft.setColor}
          group={draft.group}
          onGroup={draft.setGroup}
        />
        <SavedNote error={draft.error} saved={draft.saved} />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={draft.saving}>{t("labels.save")}</SubmitButton>
        </div>
      </Form>
      <ShowChanges
        key={draft.name.trim()}
        kind={sortingKind.label}
        id={label.id}
        button={t("labels.show-changes")}
        asked={renamed ? { name: draft.name.trim() } : null}
      />
    </>
  );
}

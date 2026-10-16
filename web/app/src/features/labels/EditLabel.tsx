import { t } from "@bugaboo/messages";
import { Alert, alertTone, Switch, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { labelsAndTagsKind } from "./change-preview.ts";
import { CombineInto } from "./CombineInto.tsx";
import { LabelForm } from "./LabelForm.tsx";
import { refreshLabels, type Label, type LabelGroup } from "./labels.ts";

export interface EditLabelProps {
  readonly label: Label;
  readonly groups: readonly LabelGroup[];
  /** The other labels, which this one can be combined into. */
  readonly others: readonly Label[];
}

/** Retiring a label, which can be taken back at once, so it is a switch. */
function RetireLabel({ label }: { readonly label: Label }) {
  const change = api.query.useMutation("patch", "/api/v1/labels/{label}", {
    onSuccess: refreshLabels,
  });
  return (
    <>
      <p {...partOf(part.muted)}>{t("labels.retire-help")}</p>
      <Switch
        label={label.retired ? t("labels.offer-again") : t("labels.retire")}
        isSelected={label.retired}
        onChange={(retired) => {
          change.mutate({
            params: { path: { label: label.id } },
            body: { version: label.version, retired },
          });
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </>
  );
}

/**
 * The editor of a label (TAG-02): its name, color and group; renaming shows what it does to the
 * issues first; retiring it; and *Combine into another*.
 */
export function EditLabel({ label, groups, others }: EditLabelProps) {
  return (
    <div {...partOf(part.stack)}>
      <LabelForm label={label} groups={groups} />
      <RetireLabel label={label} />
      <CombineInto
        kind={labelsAndTagsKind.label}
        id={label.id}
        name={label.name}
        version={label.version}
        others={others.filter((each) => each.id !== label.id)}
      />
    </div>
  );
}

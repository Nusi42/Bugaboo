import { t } from "@bugaboo/messages";
import { Alert, alertTone, ConfirmDialog, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { sortingKind, type SortingKind } from "./change-preview.ts";
import { ShowChanges } from "./ChangePreview.tsx";
import { refreshLabels } from "./labels.ts";

/** A tag or a label that another can be combined into: the version is the one that was looked at. */
export interface CombineTarget {
  readonly id: string;
  readonly name: string;
  readonly version: number;
}

export interface CombineIntoProps {
  readonly kind: SortingKind;
  readonly id: string;
  readonly name: string;
  readonly version: number;
  /** The others of the same kind, from which one is chosen. */
  readonly others: readonly CombineTarget[];
}

/** The choice that stands for no choice yet. */
const NONE = "";

/** Sends the combination of the one into the other, on the versions that were looked at. */
function useCombine(kind: SortingKind) {
  const tag = api.query.useMutation("post", "/api/v1/tags/{tag}/combine", {
    onSuccess: refreshLabels,
  });
  const label = api.query.useMutation("post", "/api/v1/labels/{label}/combine", {
    onSuccess: refreshLabels,
  });
  const used = kind === sortingKind.tag ? tag : label;
  return {
    isSuccess: used.isSuccess,
    error: used.isError ? messageOf(used.error) : null,
    send: (props: CombineIntoProps, into: CombineTarget) => {
      const body = { into: into.id, into_version: into.version, version: props.version };
      if (kind === sortingKind.tag) {
        tag.mutate({ params: { path: { tag: props.id } }, body });
      } else {
        label.mutate({ params: { path: { label: props.id } }, body });
      }
    },
  };
}

/**
 * *Combine into another*: every issue that has this one gets the other, and this one is gone. The
 * preview says which issues change, and the combination asks once more (*Show, then do*).
 */
export function CombineInto(props: CombineIntoProps) {
  const { kind, id, name, others } = props;
  const [pickedId, setPickedId] = useState(NONE);
  const combine = useCombine(kind);
  const picked = others.find((each) => each.id === pickedId);
  return (
    <div {...partOf(part.stack)}>
      <h3>{t("labels.combine-title")}</h3>
      <p {...partOf(part.muted)}>{t("labels.combine-help")}</p>
      <SelectField
        label={t("labels.combine-into")}
        choices={[
          { value: NONE, label: t("labels.combine-pick") },
          ...others.map((each) => ({ value: each.id, label: each.name })),
        ]}
        value={pickedId}
        onChange={setPickedId}
      />
      <ShowChanges
        key={pickedId}
        kind={kind}
        id={id}
        asked={picked === undefined ? null : { into: picked.id }}
      />
      {combine.error === null ? null : <Alert tone={alertTone.danger}>{combine.error}</Alert>}
      {combine.isSuccess ? <Alert tone={alertTone.success}>{t("labels.combined")}</Alert> : null}
      {picked === undefined ? null : (
        <div {...partOf(part.row)}>
          <ConfirmDialog
            triggerLabel={t("labels.combine")}
            title={t("labels.combine-dialog-title", { name, other: picked.name })}
            text={t("labels.combine-dialog-text", { name, other: picked.name })}
            confirmLabel={t("labels.combine")}
            cancelLabel={t("common.cancel")}
            onConfirm={() => {
              combine.send(props, picked);
            }}
          />
        </div>
      )}
    </div>
  );
}

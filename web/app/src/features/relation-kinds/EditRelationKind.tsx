import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, Switch, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshRelationKinds } from "../../shared/query-keys.ts";
import type { RelationKindLine } from "./relation-kinds.ts";

/**
 * The words of the kind, saved with the button on the version that was looked at, and *Retire*: no
 * new relations of the kind, while the old ones keep it (ISS-04). Retiring is a switch, because it
 * can be taken back at once.
 */
export function EditRelationKind({ kind }: { readonly kind: RelationKindLine }) {
  const [forward, setForward] = useState(kind.forward);
  const [backward, setBackward] = useState(kind.backward);
  const change = api.query.useMutation("patch", "/api/v1/relation-kinds/{relation_kind}", {
    onSuccess: refreshRelationKinds,
  });
  const path = { params: { path: { relation_kind: kind.id } } } as const;
  return (
    <div {...partOf(part.stack)}>
      <Form
        onSubmit={() => {
          change.mutate({
            ...path,
            body: { version: kind.version, forward: forward.trim(), backward: backward.trim() },
          });
        }}
      >
        <TextField
          label={t("relation-kinds.field-forward")}
          requiredMessage={t("error.field.required")}
          value={forward}
          onChange={setForward}
        />
        <TextField
          label={t("relation-kinds.field-backward")}
          requiredMessage={t("error.field.required")}
          value={backward}
          onChange={setBackward}
        />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
      <p {...partOf(part.muted)}>{t("relation-kinds.retire-help")}</p>
      <Switch
        label={kind.retired ? t("relation-kinds.offer-again") : t("relation-kinds.retire")}
        isSelected={kind.retired}
        onChange={(retired) => {
          change.mutate({ ...path, body: { version: kind.version, retired } });
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      {change.isSuccess ? <Alert tone={alertTone.success}>{t("save.saved")}</Alert> : null}
    </div>
  );
}

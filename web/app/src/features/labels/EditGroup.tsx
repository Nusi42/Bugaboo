import { t } from "@bugaboo/messages";
import { Form, SubmitButton, Switch, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshLabels, type LabelGroup } from "./labels.ts";
import { GroupOffers } from "./GroupOffers.tsx";
import { SavedNote } from "./SavedNote.tsx";

/**
 * The editor of a group of labels: its name, whether an issue gets at most one label of it, and
 * retiring it. Making it a group to pick one from is refused while an issue has two of its labels.
 */
export function EditGroup({ group }: { readonly group: LabelGroup }) {
  return (
    <div {...partOf(part.stack)}>
      {group.may_manage ? <GroupEditor group={group} /> : null}
      <GroupOffers group={group} />
    </div>
  );
}

/** The name, *Pick one* and retiring of a group: for the people who look after it. */
function GroupEditor({ group }: { readonly group: LabelGroup }) {
  const [name, setName] = useState(group.name);
  const [pickOne, setPickOne] = useState(group.pick_one);
  const change = api.query.useMutation("patch", "/api/v1/label-groups/{label_group}", {
    onSuccess: refreshLabels,
  });
  const path = { params: { path: { label_group: group.id } } } as const;
  return (
    <div {...partOf(part.stack)}>
      <Form
        onSubmit={() => {
          change.mutate({
            ...path,
            body: { version: group.version, name: name.trim(), pick_one: pickOne },
          });
        }}
      >
        <TextField
          label={t("labels.field-name")}
          requiredMessage={t("error.field.required")}
          value={name}
          onChange={setName}
        />
        <Switch label={t("labels.field-pick-one")} isSelected={pickOne} onChange={setPickOne} />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
      <p {...partOf(part.muted)}>{t("labels.retire-help")}</p>
      <Switch
        label={group.retired ? t("labels.offer-again") : t("labels.retire")}
        isSelected={group.retired}
        onChange={(retired) => {
          change.mutate({ ...path, body: { version: group.version, retired } });
        }}
      />
      <SavedNote error={change.isError ? messageOf(change.error) : null} saved={change.isSuccess} />
    </div>
  );
}

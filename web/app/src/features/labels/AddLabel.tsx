import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonType, Form, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshLabels, useLabels, type Chip } from "./labels.ts";

/** The choice that stands for no label picked yet. */
const NONE = "";

/** *Add a label*: one of the labels of the organization that the issue does not have yet. */
export function AddLabel({
  issue,
  has,
}: {
  readonly issue: string;
  readonly has: readonly Chip[];
}) {
  const [picked, setPicked] = useState(NONE);
  const labels = useLabels();
  const add = api.query.useMutation("post", "/api/v1/issues/{issue}/labels", {
    onSuccess: () => {
      refreshLabels();
      setPicked(NONE);
    },
  });
  const offered = (labels.data?.labels ?? []).filter(
    (label) => !label.retired && !has.some((each) => each.id === label.id),
  );
  if (offered.length === 0) {
    return <p {...partOf(part.muted)}>{t("labels.nothing-to-add")}</p>;
  }
  return (
    <Form
      onSubmit={() => {
        add.mutate({ params: { path: { issue } }, body: { label: picked } });
      }}
    >
      <SelectField
        label={t("labels.add-label")}
        choices={[
          { value: NONE, label: t("labels.pick-label") },
          ...offered.map((label) => ({ value: label.id, label: label.name })),
        ]}
        value={picked}
        onChange={setPicked}
      />
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} isPending={add.isPending} isDisabled={picked === NONE}>
          {t("labels.add-label-button")}
        </Button>
      </div>
    </Form>
  );
}

import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonType, Form, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { afterLabelsAndTags, useLabelsToAdd, type Chip, type Counted } from "./labels.ts";

/** The choice that stands for no label picked yet. */
const NONE = "";

/** *Add a label*: one of the labels of the organization that the issue does not have yet. */
export function AddLabel({
  issue,
  has,
  project,
}: {
  readonly issue: string;
  readonly has: readonly Chip[];
  /** The project of the issue (a short name): only the labels of the groups it offers are chosen from. */
  readonly project?: string | undefined;
}) {
  const [picked, setPicked] = useState(NONE);
  const offered = useLabelsToAdd(project, has);
  const add = api.query.useMutation("post", "/api/v1/issues/{issue}/labels", {
    onSuccess: (labelsAndTags: Counted) => {
      afterLabelsAndTags(issue, labelsAndTags);
      setPicked(NONE);
    },
  });
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

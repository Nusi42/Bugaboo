import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, WindowLayout } from "@bugaboo/ui";

import { fieldOf, messageOf } from "../../shared/errors.ts";
import { useFieldName } from "../fields/index.ts";
import { DraftOffer } from "./DraftOffer.tsx";
import { NewIssueFields, NewIssueText } from "./NewIssueFields.tsx";
import type { NewIssue } from "./use-new-issue.ts";

export interface NewIssueFormProps {
  readonly form: NewIssue;
  /** Names the form, so the main button in the footer of the window can send it. */
  readonly formId: string;
  /** Called with the readable ID and the project when an issue was made and the window is done. */
  readonly onMade: (key: string, project: string) => void;
}

/** What was refused; when it is about a field the form does not show, the field is named. */
function Refused({
  error,
  project,
}: {
  readonly error: unknown;
  readonly project: string | undefined;
}) {
  const field = fieldOf(error);
  const name = useFieldName(project, field ?? "");
  return field === undefined
    ? messageOf(error)
    : t("issues.refused-at-field", { field: name, reason: messageOf(error) });
}

/**
 * The form of a new issue in its window (ISS-11, UX §4.5): the title and the description on the
 * left, and the project, the type and the other fields at the side once the window is 880 wide.
 * What is written is kept as a draft, in the browser and on the server, until the issue is made;
 * *Create the issue and start another* keeps the project and the type. The buttons are in the footer of the
 * window (`NewIssueActions`).
 */
export function NewIssueForm({ form, formId, onMade }: NewIssueFormProps) {
  const { project, values, offered } = form;
  // A refusal about a field is shown at that field, if the form has it; any other at the bottom.
  const refusedField = fieldOf(form.error);
  const atField = refusedField !== undefined && form.layout.shows(refusedField);
  const refusal = atField ? { field: refusedField, message: messageOf(form.error) } : undefined;
  return (
    <Form
      id={formId}
      onSubmit={() => {
        form.create(false, onMade);
      }}
    >
      <WindowLayout fields={<NewIssueFields form={form} refusal={refusal} />}>
        {offered === null ? null : (
          <DraftOffer
            draft={offered}
            now={form.now}
            onContinue={() => {
              form.continueDraft(offered);
            }}
            onDrop={form.dropDraft}
          />
        )}
        {form.lastKey === null ? null : (
          <Alert tone={alertTone.success}>{t("issues.created", { key: form.lastKey })}</Alert>
        )}
        {project === undefined ? null : (
          <NewIssueText
            values={values}
            project={project.shortName}
            layout={form.layout}
            onChange={form.change}
          />
        )}
        {form.error === null || atField ? null : (
          <Alert tone={alertTone.danger}>
            <Refused error={form.error} project={project?.shortName} />
          </Alert>
        )}
      </WindowLayout>
    </Form>
  );
}

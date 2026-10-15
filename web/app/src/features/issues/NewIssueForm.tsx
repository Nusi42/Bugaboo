import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, partOf, part } from "@bugaboo/ui";

import { fieldOf, messageOf } from "../../shared/errors.ts";
import { useFieldName } from "../fields/index.ts";
import { DraftOffer } from "./DraftOffer.tsx";
import { NewIssueActions } from "./NewIssueActions.tsx";
import { NewIssueFields } from "./NewIssueFields.tsx";
import { ProjectChoice } from "./ProjectChoice.tsx";
import { useNewIssue } from "./use-new-issue.ts";

export interface NewIssueFormProps {
  /** Called with the readable ID and the project when an issue was made and the dialog is done. */
  readonly onMade: (key: string, project: string) => void;
  readonly onClose: () => void;
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
 * The form of a new issue in its dialog (ISS-11, UX §4.5): project, type, then the fields. What is
 * written is kept as a draft, in the browser and on the server, until the issue is made;
 * *Create and start another* keeps the project and the type.
 */
export function NewIssueForm({ onMade, onClose }: NewIssueFormProps) {
  const form = useNewIssue();
  const { project, values, offered } = form;
  // A refusal about a field is shown at that field, if the form has it; any other at the bottom.
  const refusedField = fieldOf(form.error);
  const atField = refusedField !== undefined && form.layout.shows(refusedField);
  return (
    <Form
      onSubmit={() => {
        form.create(false, onMade);
      }}
    >
      <h2 {...partOf(part.issueSectionTitle)}>{t("issues.new-title")}</h2>
      <ProjectChoice name={project?.name} shortName={form.address} onPick={form.pick} />
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
        <NewIssueFields
          values={values}
          types={form.types}
          project={project.shortName}
          layout={form.layout}
          refusal={atField ? { field: refusedField, message: messageOf(form.error) } : undefined}
          onChange={form.change}
        />
      )}
      {form.error === null || atField ? null : (
        <Alert tone={alertTone.danger}>
          <Refused error={form.error} project={project?.shortName} />
        </Alert>
      )}
      <p {...partOf(part.muted)}>{t("issues.draft-kept")}</p>
      <NewIssueActions form={form} onMade={onMade} onClose={onClose} />
    </Form>
  );
}

import { t } from "@bugaboo/messages";
import { Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";

import { ColorChoice } from "../../shared/ColorChoice.tsx";
import { labelsAndTagsKind } from "./change-preview.ts";
import { ShowChanges } from "./ChangePreview.tsx";
import type { TagLine } from "./labels.ts";
import { SavedNote } from "./SavedNote.tsx";
import { TagProjectsField } from "./TagProjectsField.tsx";
import { useTagDraft } from "./use-tag-draft.ts";

/** The name, color and projects of a tag, saved on the version that was looked at. */
export function TagForm({ tag }: { readonly tag: TagLine }) {
  const draft = useTagDraft(tag);
  const renamed = draft.name.trim() !== "" && draft.name.trim() !== tag.name;
  return (
    <>
      <Form
        onSubmit={() => {
          draft.save();
        }}
      >
        <TextField
          label={t("labels.field-name")}
          requiredMessage={t("error.field.required")}
          value={draft.name}
          onChange={draft.setName}
        />
        <ColorChoice
          legend={t("labels.field-color", { name: tag.name })}
          value={draft.color}
          onChange={draft.setColor}
        />
        <TagProjectsField
          everywhere={draft.everywhere}
          onEverywhere={draft.setEverywhere}
          projects={draft.projects}
          setProjects={draft.setProjects}
        />
        <SavedNote error={draft.error} saved={draft.saved} />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={draft.saving}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
      <ShowChanges
        key={draft.name.trim()}
        kind={labelsAndTagsKind.tag}
        id={tag.id}
        button={t("labels.show-changes")}
        asked={renamed ? { name: draft.name.trim() } : null}
      />
    </>
  );
}

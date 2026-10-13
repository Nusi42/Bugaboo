import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonType, Form, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshLabels, useTagSuggestions } from "./labels.ts";

export interface AddTagProps {
  /** The issue, by its readable ID. */
  readonly issue: string;
  /** The project of the issue (its short name): the tags to offer are those of the project. */
  readonly project: string;
}

interface SuggestionsProps {
  readonly project: string;
  readonly text: string;
  readonly pick: (name: string) => void;
}

/** The tags of the project that fit the words typed so far, each a button that fills them in. */
function Suggestions({ project, text, pick }: SuggestionsProps) {
  const found = useTagSuggestions(project, text);
  const tags = text.trim() === "" ? [] : (found.data?.tags ?? []);
  if (tags.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.chips)} aria-label={t("labels.tag-suggestions")}>
      {tags.map((tag) => (
        <li key={tag.id}>
          <Button
            onPress={() => {
              pick(tag.name);
            }}
          >
            {tag.name}
          </Button>
        </li>
      ))}
    </ul>
  );
}

/**
 * *Add a tag*: the words typed offer the tags of the project that fit; a tag that does not exist
 * yet is made by adding it (TAG-01).
 */
export function AddTag({ issue, project }: AddTagProps) {
  const [text, setText] = useState("");
  const add = api.query.useMutation("post", "/api/v1/issues/{issue}/tags", {
    onSuccess: () => {
      refreshLabels();
      setText("");
    },
  });
  const empty = text.trim() === "";
  return (
    <Form
      onSubmit={() => {
        add.mutate({ params: { path: { issue } }, body: { name: text.trim() } });
      }}
    >
      <TextField label={t("labels.add-tag")} value={text} onChange={setText} />
      <Suggestions project={project} text={text} pick={setText} />
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} isPending={add.isPending} isDisabled={empty}>
          {t("labels.add-tag-button")}
        </Button>
      </div>
    </Form>
  );
}

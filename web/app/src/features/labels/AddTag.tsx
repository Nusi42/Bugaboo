import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonType, Form, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { afterSorting, useTagSuggestions, type Counted } from "./labels.ts";

export interface AddTagProps {
  /** The issue, by its readable ID. */
  readonly issue: string;
  /** The project of the issue (its short name): the tags to offer are those of the project. */
  readonly project: string;
  /** The type of the issue: its level for *Tags* decides what is offered. */
  readonly issueType: string;
}

interface SuggestionsProps {
  readonly project: string;
  readonly issueType: string;
  readonly text: string;
  readonly pick: (name: string) => void;
}

/** The tags of the project that fit the words typed so far, each a button that fills them in. */
function Suggestions({ project, issueType, text, pick }: SuggestionsProps) {
  const found = useTagSuggestions(project, issueType, text);
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
export function AddTag({ issue, project, issueType }: AddTagProps) {
  const [text, setText] = useState("");
  const add = api.query.useMutation("post", "/api/v1/issues/{issue}/tags", {
    onSuccess: (sorting: Counted) => {
      afterSorting(issue, sorting);
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
      <Suggestions project={project} issueType={issueType} text={text} pick={setText} />
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} isPending={add.isPending} isDisabled={empty}>
          {t("labels.add-tag-button")}
        </Button>
      </div>
    </Form>
  );
}

import { t } from "@bugaboo/messages";
import { Button, buttonLook, buttonType, Form, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useTagSuggestions } from "./labels.ts";

export interface AddTagProps {
  /** The project of the issue (its short name): the tags to offer are those of the project. */
  readonly project: string;
  /** The type of the issue: its level for *Tags* decides what is offered. */
  readonly issueType: string;
  /** Adds the tag with this name; says whether it was added. */
  readonly onAdd: (name: string) => Promise<boolean>;
  /** A change is on its way. */
  readonly pending: boolean;
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
export function AddTag({ project, issueType, onAdd, pending }: AddTagProps) {
  const [text, setText] = useState("");
  const [open, setOpen] = useState(false);
  const empty = text.trim() === "";
  // A row with no tag in it is one quiet *Add tag* until the person wants to add one (UX §4.4).
  if (!open) {
    return (
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("labels.add-tag-button")}
        </Button>
      </div>
    );
  }
  return (
    <Form
      onSubmit={() => {
        void onAdd(text.trim()).then((added) => {
          if (added) {
            setText("");
            setOpen(false);
          }
        });
      }}
    >
      <TextField hideLabel label={t("labels.add-tag")} value={text} onChange={setText} />
      <Suggestions project={project} issueType={issueType} text={text} pick={setText} />
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} isPending={pending} isDisabled={empty}>
          {t("labels.add-tag-button")}
        </Button>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setText("");
            setOpen(false);
          }}
        >
          {t("common.cancel")}
        </Button>
      </div>
    </Form>
  );
}

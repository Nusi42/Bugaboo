import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  MarkdownEditor,
  SubmitButton,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { pageSize } from "../../shared/paged.ts";
import { refreshHistory } from "../../shared/query-keys.ts";
import { mentionBeingTyped, withMention } from "./mention-text.ts";

/** No image from another host in a comment being written: the hosts come with the issue page. */
const noImageHosts: readonly string[] = [];

/**
 * What the writer may name after the `@` they are typing: people and groups for someone who sees
 * names, groups and roles for someone who sees roles only. A choice puts the whole handle in.
 */
function MentionChoices({
  issue,
  typed,
  onChoose,
}: {
  readonly issue: string;
  readonly typed: string;
  readonly onChoose: (handle: string) => void;
}) {
  const choices = api.query.useQuery("get", "/api/v1/issues/{issue}/mention-choices", {
    params: { path: { issue }, query: { search: typed, limit: pageSize.short } },
  });
  const items = choices.data?.items ?? [];
  if (items.length === 0) {
    return null;
  }
  return (
    <fieldset {...partOf(part.choiceGroup)}>
      <legend {...partOf(part.choiceGroupLegend)}>{t("comments.mention-choices")}</legend>
      {items.map((choice) => (
        <Button
          key={choice.handle}
          look={buttonLook.quiet}
          onPress={() => {
            onChoose(choice.handle);
          }}
        >
          {choice.name}
        </Button>
      ))}
    </fieldset>
  );
}

/** *Write a comment*: the Markdown editor with its preview, and the `@` choices below it. */
export function CommentComposer({ issue }: { readonly issue: string }) {
  const [text, setText] = useState("");
  const add = api.query.useMutation("post", "/api/v1/issues/{issue}/comments", {
    onSuccess: () => {
      refreshHistory();
      setText("");
    },
  });
  const typed = mentionBeingTyped(text);
  return (
    <Form
      onSubmit={() => {
        add.mutate({ params: { path: { issue } }, body: { text } });
      }}
    >
      <MarkdownEditor
        label={t("comments.write")}
        labels={markdownLabels()}
        value={text}
        onChange={setText}
        imageHosts={noImageHosts}
      />
      <p {...partOf(part.muted)}>{t("comments.write-help")}</p>
      {typed === undefined ? null : (
        <MentionChoices
          issue={issue}
          typed={typed}
          onChoose={(handle) => {
            setText(withMention(text, handle));
          }}
        />
      )}
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={add.isPending}>{t("comments.add")}</SubmitButton>
      </div>
    </Form>
  );
}

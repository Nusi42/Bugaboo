import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  MarkdownView,
  partOf,
  part,
} from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { refreshHistory } from "../../shared/query-keys.ts";

/** How many mentions the start page shows: the newest few, unread first. */
const SHOWN = 10;

/** One comment that mentions the person: who, where, when, and the text. */
function Mention({ item }: { readonly item: Schema<"MentionedComment"> }) {
  return (
    <li {...partOf(part.item)}>
      <div {...partOf(part.stack)}>
        <span {...partOf(part.historyWhen)}>
          {t("mentions.in", { who: personName(item.author), issue: item.issue.key })}
        </span>
        <span {...partOf(part.historyWhen)}>{formatDateTime(item.at)}</span>
        {item.unread ? <strong {...partOf(part.strong)}>{t("mentions.new")}</strong> : null}
        <MarkdownView html={item.html} />
      </div>
    </li>
  );
}

/**
 * *Mentions* on the start page (UX §4.1): the comments where someone named the person, by name, by
 * a group or by a role, unread first. Only comments under issues the person may see are listed.
 */
export function MentionsBlock() {
  const list = api.query.useQuery("get", "/api/v1/me/mentions", {
    params: { query: { limit: SHOWN } },
  });
  const read = api.query.useMutation("post", "/api/v1/me/mentions/read", {
    onSuccess: refreshHistory,
  });
  const items = list.data?.items ?? [];
  const unread = items.filter((item) => item.unread).map((item) => item.comment);
  return (
    <Card heading={t("mentions.title")}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {items.length === 0 && list.isSuccess ? (
        <p {...partOf(part.muted)}>{t("mentions.nothing")}</p>
      ) : null}
      <ul {...partOf(part.list)}>
        {items.map((item) => (
          <Mention key={item.comment} item={item} />
        ))}
      </ul>
      {unread.length === 0 ? null : (
        <div {...partOf(part.row)}>
          <span {...partOf(part.muted)}>{t("mentions.unread", { count: unread.length })}</span>
          <Button
            look={buttonLook.quiet}
            isPending={read.isPending}
            onPress={() => {
              read.mutate({ body: { comments: unread } });
            }}
          >
            {t("mentions.mark-read")}
          </Button>
        </div>
      )}
    </Card>
  );
}

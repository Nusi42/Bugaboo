import { t } from "@bugaboo/messages";
import { partOf, part, SentenceWithValue } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatDate, formatDateTime } from "../../../shared/format.ts";
import type { AccessKey } from "./access-keys.ts";

/** What a key shows of its start: `bb_` and the public part, the one text of the lists of access keys and of agent keys. */
const keyStart = (prefix: string): string => `bb_${prefix}`;

/** When the key runs out, or ran out. */
function RunsOut({ item }: { readonly item: AccessKey }) {
  const day = item.expires_at;
  if (day === undefined || day === null) {
    return null;
  }
  // The day on the clock of the region, the clock the box of the form counts the days by.
  const when = formatDate(day);
  return item.expired === true ? (
    <span {...partOf(part.accessKeysExpired)}>{t("access-keys.row.expired", { when })}</span>
  ) : (
    <span {...partOf(part.muted)}>{t("access-keys.row.expires", { when })}</span>
  );
}

/** When a key, or an agent that signs in with one, was last used: *Never used* while it never was. */
export function LastUsed({ at }: { readonly at: string | null | undefined }) {
  return (
    <span {...partOf(part.muted)}>
      {at === undefined || at === null
        ? t("access-keys.row.never-used")
        : t("access-keys.row.last-used", { when: formatDateTime(at) })}
    </span>
  );
}

export interface KeyFactsProps {
  readonly item: AccessKey;
  /** What only this kind of key says, between the start of the key and when it runs out. */
  readonly children?: ReactNode;
}

/**
 * What a list of keys says of one key: its name, how it starts, when it runs out and when it was last used.
 * The one part of the rows of access keys and of agent keys, so a key reads the same in both.
 */
export function KeyFacts({ item, children }: KeyFactsProps) {
  const start = keyStart(item.prefix ?? "");
  return (
    <span {...partOf(part.accessKeysItemText)}>
      <span {...partOf(part.itemTitle)}>{item.name ?? ""}</span>
      <SentenceWithValue sentence={t("access-keys.row.prefix", { prefix: start })} value={start}>
        {(value) => <span {...partOf(part.accessKeysPrefix)}>{value}</span>}
      </SentenceWithValue>
      {children}
      <RunsOut item={item} />
      <LastUsed at={item.last_used_at} />
    </span>
  );
}

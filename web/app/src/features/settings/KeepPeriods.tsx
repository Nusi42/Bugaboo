import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { refreshRoutes } from "../../shared/query-keys.ts";
import type { DataKind, KeepRule } from "./keep-rules.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";
import type { NoteFor } from "./SettingControl.tsx";
import { WouldDelete } from "./WouldDelete.tsx";

/** The kinds whose period is one setting and has an end, by the key of that setting: those a box changes here. */
function changeable(rules: readonly KeepRule[]): ReadonlyMap<string, DataKind> {
  return new Map(
    rules.flatMap((rule) =>
      rule.fixed_end && typeof rule.setting === "string"
        ? [[rule.setting, rule.kind] as const]
        : [],
    ),
  );
}

/** The note of the box of the period of `kind`: what the number typed would delete. */
function periodNote(kind: DataKind): NoteFor {
  return (count) => <WouldDelete kind={kind} count={count} />;
}

/**
 * *Change a keep period*: the settings of the periods in the generic form, each box saying what the number typed would
 * delete before it is saved. After a change the rules and the counts are read again.
 */
export function KeepPeriods({ rules }: { readonly rules: readonly KeepRule[] }) {
  const kinds = changeable(rules);
  const notes = Object.fromEntries(
    [...kinds].map((entry: readonly [string, DataKind]) => [entry[0], periodNote(entry[1])]),
  );
  return (
    <Card heading={t("keep-rules.periods")} intro={t("keep-rules.periods-intro")}>
      <OrganizationTopic
        topics={[...kinds.keys()].join(",")}
        searchable={false}
        notes={notes}
        onSettled={() => {
          // The rules, the counts past a period and what a change would delete are read again.
          refreshRoutes(["/api/v1/admin/keep-rules"]);
        }}
      />
    </Card>
  );
}

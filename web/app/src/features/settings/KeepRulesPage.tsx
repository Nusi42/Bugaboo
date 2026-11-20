import { t } from "@bugaboo/messages";
import { Card, cardLook } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { keepRulesOptions, type KeepRule } from "./keep-rules.ts";
import { KeepPeriods } from "./KeepPeriods.tsx";
import { KeepRulesTables } from "./KeepRulesTables.tsx";
import { OldRecords } from "./OldRecords.tsx";

/**
 * *Organization settings › Privacy › Keep and delete rules* (PRIV-05, UX §5.7, ADR-0045): every kind of data in two
 * tables, what Bugaboo deletes by itself and what an administrator deletes; the periods, each saying what a change
 * would delete before it is saved; and, for each kind an administrator deletes, what is past its period with *Delete
 * them*. The rules come from the server's list, so no period is written in the browser.
 */
export function KeepRulesPage() {
  usePageTitle(t("nav.keep-rules"));
  const rules = useQuery(keepRulesOptions());
  return (
    <>
      <Card
        heading={t("nav.keep-rules")}
        level={1}
        look={cardLook.pageHead}
        intro={t("keep-rules.intro")}
      />
      <QueryGate error={rules.error} data={rules.data?.rules} loading={null}>
        {(found: readonly KeepRule[]) => (
          <>
            <KeepRulesTables rules={found} />
            <KeepPeriods rules={found} />
            {found
              .filter((rule) => rule.group === "by-an-administrator")
              .map((rule) => (
                <OldRecords key={rule.kind} kind={rule.kind} />
              ))}
          </>
        )}
      </QueryGate>
    </>
  );
}

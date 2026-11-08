import { t } from "@bugaboo/messages";
import { Alert, alertTone, Switch, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { QueryGate } from "../shell/index.ts";
import { useMarkField, useNeverSentMarks } from "./ai.ts";

/**
 * *Never sent to AI* for the built-in fields (AI-03): one switch for each field of the catalogue. A
 * field that is marked stays out of every answer of the assistant, of AI apps and of agents, also for
 * the people who can read it. The fields an administrator made have the same switch on their page.
 */
export function NeverSentFields() {
  const marks = useNeverSentMarks();
  const mark = useMarkField();
  return (
    <QueryGate error={marks.error} data={marks.data} loading={null}>
      {(shown) => (
        <div {...partOf(part.stack)}>
          <p {...partOf(part.muted)}>{t("ai-setup.never-sent-help")}</p>
          <ul {...partOf(part.list, { variants: ["tight"] })} aria-label={t("ai-setup.never-sent")}>
            {shown.fields.map((each) => (
              <li key={each.field}>
                <Switch
                  label={fieldNameOf({ code: each.field })}
                  isSelected={each.marked}
                  onChange={(marked) => {
                    mark.mutate({
                      params: { path: { field: each.field } },
                      body: { marked, version: each.version },
                    });
                  }}
                />
              </li>
            ))}
          </ul>
          {mark.isError ? <Alert tone={alertTone.danger}>{messageOf(mark.error)}</Alert> : null}
        </div>
      )}
    </QueryGate>
  );
}

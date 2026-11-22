import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ChoiceGroup, choiceGroupLook, partOf, part } from "@bugaboo/ui";

import { formatCalendarDay } from "../../shared/format.ts";
import { ConfirmPasswordField } from "../account/index.ts";

/** What happens to the comments of a removed person (`RemovedTexts` of the API). */
export type CommentsChoiceValue = Schema<"RemovedTexts">;

export interface RemovalAnswersProps {
  readonly name: string;
  readonly comments: CommentsChoiceValue;
  readonly onComments: (value: CommentsChoiceValue) => void;
  readonly password: string;
  readonly onPassword: (value: string) => void;
}

/**
 * What the person who removes answers: what happens to the comments, keep them shown as *Former user* or delete them
 * (ADR-0044, decision 7), and the password again, since a removal cannot be undone.
 */
export function RemovalAnswers({
  name,
  comments,
  onComments,
  password,
  onPassword,
}: RemovalAnswersProps) {
  return (
    <>
      <ChoiceGroup
        legend={t("removal.comments", { name })}
        look={choiceGroupLook.radios}
        choices={[
          { value: "keep", label: t("setting.privacy.removed-texts.keep") },
          { value: "delete", label: t("setting.privacy.removed-texts.delete") },
        ]}
        value={comments}
        onChange={onComments}
      />
      <ConfirmPasswordField value={password} onChange={onPassword} />
    </>
  );
}

type Plan = Schema<"RemovalPlanBody">;
type KeptByLaw = Schema<"KeptByLawBody">;

/** One kind of record the law keeps: how many, until which day, or why no day can be named. */
function keptText(kept: KeptByLaw): string {
  const { title } = kept;
  const rows = kept.rows ?? undefined;
  const until = kept.until ?? undefined;
  if (until !== undefined && rows !== undefined) {
    return t("removal.rows-until", { title, count: rows, date: formatCalendarDay(until) });
  }
  if (until !== undefined) {
    return t("removal.until", { title, date: formatCalendarDay(until) });
  }
  return (
    kept.no_fixed_end ?? (rows === undefined ? title : t("removal.rows", { title, count: rows }))
  );
}

/** One line of a list of the plan: its words, and the kind it names, which is its key. */
interface PlanLine {
  readonly key: string;
  readonly text: string;
}

/** A list of the plan under its heading; *Nothing.* where it is empty. */
function PlanList({
  heading,
  lines,
}: {
  readonly heading: string;
  readonly lines: readonly PlanLine[];
}) {
  return (
    <section {...partOf(part.stack)}>
      <h3 {...partOf(part.formSectionTitle)}>{heading}</h3>
      {lines.length === 0 ? (
        <p {...partOf(part.muted)}>{t("removal.nothing")}</p>
      ) : (
        <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={heading}>
          {lines.map((line) => (
            <li key={line.key} {...partOf(part.item)}>
              {line.text}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** A kind of record with how many of its rows name the person; the server names each kind once in a list. */
function rowsLine(kind: Schema<"PlanKindBody">): PlanLine {
  return { key: kind.title, text: t("removal.rows", { title: kind.title, count: kind.rows }) };
}

/** A kind the law keeps, by its row of the keep rules. */
function keptLine(kept: KeptByLaw): PlanLine {
  return { key: kept.kind, text: keptText(kept) };
}

/**
 * The three lists of the plan of a removal, as the server wrote them from the walk of the person's data and the keep
 * rules (ADR-0044, decision 6): what is deleted now, what stays shown as *Former user*, and what the law keeps, with the
 * name, until when. The titles are the server's words; nothing here decides what goes.
 */
export function PlanLists({ plan }: { readonly plan: Plan }) {
  return (
    <>
      <PlanList heading={t("removal.deleted-now")} lines={plan.deleted_now.map(rowsLine)} />
      <PlanList heading={t("removal.stays")} lines={plan.kept_as_former_user.map(rowsLine)} />
      <PlanList heading={t("removal.kept-by-law")} lines={plan.kept_by_law.map(keptLine)} />
    </>
  );
}

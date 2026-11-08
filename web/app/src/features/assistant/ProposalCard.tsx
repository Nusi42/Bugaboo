import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Icon,
  iconName,
  iconSize,
  IssueId,
  Priority,
  partOf,
  part,
} from "@bugaboo/ui";
import type { ReactNode } from "react";

import { changeArrowText, changeLineText } from "../../shared/change-line.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { changeValueText } from "./change-value.ts";

/** One field a proposed change touches: the value now and the value after (none for a field that is new or taken away). */
export interface CardChange {
  readonly field: string;
  readonly before?: unknown;
  readonly after?: unknown;
  /** The same change in the words of the history (status by its name, people as the reader may see them), when the server has them. */
  readonly words?: Schema<"FieldChangeBody"> | undefined;
}

/** Where a card stands: waiting for the person, saving, saved, dropped, or refused with a reason. */
export type CardState =
  | { readonly kind: "open" }
  | { readonly kind: "saving" }
  | { readonly kind: "saved"; readonly problem?: string }
  | { readonly kind: "undone" }
  | { readonly kind: "discarded" }
  | { readonly kind: "refused"; readonly problem: string };

export interface ProposalCardProps {
  /** The issue the change is about, by its readable ID; a card about no issue (a new one) names it in `subject`. */
  readonly issue?: string | undefined;
  /** What the change is about when it is no issue yet: *A new issue in WEB*. */
  readonly subject?: string | undefined;
  readonly changes: readonly CardChange[];
  readonly state: CardState;
  /** Who proposed it, if the card says so; the assistant's own cards leave it out. */
  readonly source?: ReactNode;
  readonly onApply: () => void;
  readonly onDiscard: () => void;
  /** After *Apply*, the way back for as long as it is on offer; none when it cannot be taken back or the time is over. */
  readonly onUndo?: (() => void) | undefined;
}

/** The priorities a change can name, with the words the list of issues uses. */
const PRIORITIES = ["low", "normal", "high", "critical"] as const;

/** A priority as the lists of issues draw it, or none when the value is not one. */
function priorityOf(value: unknown): (typeof PRIORITIES)[number] | undefined {
  return PRIORITIES.find((each) => each === value);
}

/** A change of priority drawn with the priority component of the app: the old one, then the new one. */
function PriorityChange({ before, after }: { readonly before: unknown; readonly after: unknown }) {
  const was = priorityOf(before);
  const now = priorityOf(after);
  return (
    <>
      <span>{fieldNameOf({ code: "priority" })}</span>
      {was === undefined ? null : <Priority priority={was} label={t(`priority.${was}`)} />}
      <Icon name={iconName.next} size={iconSize.small} />
      {now === undefined ? null : <Priority priority={now} label={t(`priority.${now}`)} />}
    </>
  );
}

/** One line of the change: *Priority: Normal → High*, or after *Apply* what it is now. */
function ChangeLine({ change, saved }: { readonly change: CardChange; readonly saved: boolean }) {
  const words = change.words;
  if (change.field === "priority" && !saved && priorityOf(change.after) !== undefined) {
    return (
      <li {...partOf(part.proposalChange, { variants: ["values"] })}>
        <PriorityChange before={change.before} after={change.after} />
      </li>
    );
  }
  if (words !== undefined && saved) {
    return (
      <li {...partOf(part.proposalChange)}>
        <WithWording>{(wording) => changeLineText(wording, words)}</WithWording>
      </li>
    );
  }
  if (words !== undefined) {
    return (
      <li {...partOf(part.proposalChange)}>
        <WithWording>{(wording) => changeArrowText(wording, words)}</WithWording>
      </li>
    );
  }
  return (
    <li {...partOf(part.proposalChange)}>
      {t("assistant.card.change", {
        field: fieldNameOf({ code: change.field }),
        before: changeValueText(change.field, change.before),
        after: changeValueText(change.field, change.after),
      })}
    </li>
  );
}

/** What stands in place of the buttons once the person decided. */
function Outcome({ state, onUndo }: Pick<ProposalCardProps, "state" | "onUndo">) {
  if (state.kind === "saved") {
    return (
      <>
        <div {...partOf(part.row)}>
          <output>{t("assistant.card.saved")}</output>
          {onUndo === undefined ? null : (
            <Button look={buttonLook.quiet} onPress={onUndo}>
              {t("common.undo")}
            </Button>
          )}
        </div>
        {state.problem === undefined ? null : (
          <Alert tone={alertTone.danger}>{state.problem}</Alert>
        )}
      </>
    );
  }
  return (
    <output {...partOf(part.muted)}>
      {state.kind === "undone" ? t("assistant.card.undone-note") : t("assistant.card.discarded")}
    </output>
  );
}

/** What stands under an open card: the sentence that nothing is saved yet, a refusal, and the two answers. */
function Choice({
  state,
  onApply,
  onDiscard,
}: Pick<ProposalCardProps, "state" | "onApply" | "onDiscard">) {
  const saving = state.kind === "saving";
  return (
    <>
      <p {...partOf(part.muted)}>{t("assistant.card.nothing-saved")}</p>
      {state.kind === "refused" ? <Alert tone={alertTone.danger}>{state.problem}</Alert> : null}
      <div {...partOf(part.row)}>
        <Button look={buttonLook.primary} isPending={saving} onPress={onApply}>
          {t("assistant.card.apply")}
        </Button>
        <Button isDisabled={saving} onPress={onDiscard}>
          {t("assistant.card.discard")}
        </Button>
      </div>
    </>
  );
}

/**
 * A change the assistant proposes (UX §4.12, ADR-0013): the issue, each field with the value now
 * and the value after, the sentence that nothing is saved until the person applies it, and *Apply*
 * and *Discard*. Made once and used for every kind of tool, and for the proposals of standing rules.
 * It draws what it is given and decides nothing: saving is the caller's.
 */
export function ProposalCard({
  issue,
  subject,
  changes,
  state,
  source,
  onApply,
  onDiscard,
  onUndo,
}: ProposalCardProps) {
  const decided = state.kind === "saved" || state.kind === "undone" || state.kind === "discarded";
  return (
    <section {...partOf(part.proposalCard)} aria-label={t("assistant.card.title")}>
      <div {...partOf(part.proposalHead)}>
        <span {...partOf(part.muted)}>
          {state.kind === "saved" || state.kind === "undone"
            ? t("assistant.card.title-saved")
            : t("assistant.card.title")}
        </span>
        {source}
      </div>
      {issue === undefined ? (
        <span>{subject}</span>
      ) : (
        <span>
          <IssueId id={issue} />
        </span>
      )}
      <ul {...partOf(part.list, { variants: ["tight"] })}>
        {changes.map((change) => (
          <ChangeLine key={change.field} change={change} saved={decided} />
        ))}
      </ul>
      {decided ? (
        <Outcome state={state} onUndo={onUndo} />
      ) : (
        <Choice state={state} onApply={onApply} onDiscard={onDiscard} />
      )}
    </section>
  );
}

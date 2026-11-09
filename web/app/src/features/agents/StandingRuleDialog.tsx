import { t } from "@bugaboo/messages";
import { AskDialog, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { SavedSearchChoice } from "../search/index.ts";
import { ActionError } from "./ActionError.tsx";
import { useCreateRule } from "./agents.ts";
import { RuleEnds } from "./RuleEnds.tsx";
import { RuleWayFields } from "./RuleWayFields.tsx";
import { RuleWhere } from "./RuleWhere.tsx";
import {
  emptyRuleDraft,
  ruleBody,
  ruleWaitsFor,
  type DraftProps,
  type RuleDraft,
} from "./rule-draft.ts";

/** The fields of the window. */
function Fields({
  draft,
  onChange,
  searchName,
}: DraftProps & { readonly searchName: string | undefined }) {
  return (
    <div {...partOf(part.stack, { variants: ["wide-hints"] })}>
      {searchName === undefined ? (
        <SavedSearchChoice
          label={t("agents.rule-search")}
          value={draft.search}
          onChange={(search) => {
            onChange({ search });
          }}
        />
      ) : (
        <p {...partOf(part.text)}>{t("agents.rule-search-named", { search: searchName })}</p>
      )}
      <RuleWhere draft={draft} onChange={onChange} />
      <RuleWayFields draft={draft} onChange={onChange} />
      <RuleEnds draft={draft} onChange={onChange} />
    </div>
  );
}

export interface StandingRuleDialogProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Where the window starts: the saved search, the agent. */
  readonly start?: Partial<RuleDraft>;
  /** The name of the saved search the window starts with: the window says which search it is for and does not ask. */
  readonly searchName?: string;
}

/** The window starts empty. */
const noStart: Partial<RuleDraft> = {};

/**
 * The window of a standing rule (UX §4.17, §6.3): the saved search, the agent, what it does with
 * what it finds (propose changes, or hand over each issue it finds with the powers of a hand-over;
 * and create issues in a project), and when it runs out (30 days, at most 90, or never where an
 * administrator allows it).
 */
export function StandingRuleDialog({
  isOpen,
  onOpenChange,
  start = noStart,
  searchName,
}: StandingRuleDialogProps) {
  const [draft, setDraft] = useState<RuleDraft>(() => emptyRuleDraft(start));
  const create = useCreateRule();
  const waits = ruleWaitsFor(draft);
  return (
    <>
      <AskDialog
        isOpen={isOpen}
        onOpenChange={onOpenChange}
        title={t("agents.rule-title")}
        text={t("agents.rule-text")}
        confirmLabel={t("agents.rule-confirm")}
        cancelLabel={t("common.cancel")}
        confirmDisabled={waits !== undefined || create.isPending}
        {...(waits === undefined ? {} : { confirmHint: t(waits) })}
        onConfirm={() => {
          create.mutate({ body: ruleBody(draft) });
        }}
      >
        <Fields
          draft={draft}
          searchName={searchName}
          onChange={(change) => {
            setDraft((was) => ({ ...was, ...change }));
          }}
        />
      </AskDialog>
      <ActionError error={create.error} />
    </>
  );
}

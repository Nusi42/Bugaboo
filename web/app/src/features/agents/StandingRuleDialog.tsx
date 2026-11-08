import { agentRuleDaysDefault, agentRuleDaysMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  AskDialog,
  Checkbox,
  ChoiceGroup,
  choiceGroupLook,
  SelectField,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { SavedSearchChoice } from "../search/index.ts";
import { ActionError } from "./ActionError.tsx";
import { listedAgents, useCreateRule, useMyAgentList } from "./agents.ts";
import { projectKey } from "./project-choices.ts";
import { ProjectSelect } from "./ProjectSelect.tsx";
import {
  emptyRuleDraft,
  offeredFields,
  ruleBody,
  ruleFieldText,
  ruleWaitsFor,
  withField,
  daysAreRight,
  type DraftProps,
  type RuleDraft,
} from "./rule-draft.ts";

/** The agents of the person that can work, as the choices of the window. */
function AgentSelect({ draft, onChange }: DraftProps) {
  const agents = listedAgents(useMyAgentList().data?.items).filter(
    (agent) => agent.state === "active",
  );
  return (
    <SelectField
      label={t("agents.rule-agent")}
      value={draft.agent}
      onChange={(agent) => {
        onChange({ agent });
      }}
      choices={[
        { value: "", label: t("agents.rule-choose-agent") },
        ...agents.map((agent) => ({ value: agent.id, label: agent.name })),
      ]}
    />
  );
}

/** The fields of the short list, one box each, for a rule that changes fields. */
function FieldBoxes({ draft, onChange }: DraftProps) {
  return (
    <fieldset {...partOf(part.agentWorkGroup)}>
      <legend {...partOf(part.agentWorkGroupName)}>{t("agents.rule-fields")}</legend>
      {offeredFields.map((field) => (
        <Checkbox
          key={field}
          label={ruleFieldText(field)}
          isSelected={draft.fields.includes(field)}
          onChange={(on) => {
            onChange({ fields: withField(draft.fields, field, on) });
          }}
        />
      ))}
    </fieldset>
  );
}

/** What the rule may do: only propose, or change the fields of the short list, and create issues in a project. */
function Powers({ draft, onChange }: DraftProps) {
  return (
    <div {...partOf(part.stack)}>
      <ChoiceGroup
        legend={t("agents.rule-may")}
        look={choiceGroupLook.radios}
        choices={[
          { value: "propose", label: t("agents.rule-propose") },
          { value: "change", label: t("agents.rule-change") },
        ]}
        value={draft.power}
        onChange={(power) => {
          onChange({ power });
        }}
      />
      {draft.power === "change" ? <FieldBoxes draft={draft} onChange={onChange} /> : null}
      <p {...partOf(part.muted)}>{t("agents.rule-never")}</p>
      <Checkbox
        label={t("agents.rule-creates")}
        isSelected={draft.createIssues}
        onChange={(on) => {
          onChange({ createIssues: on });
        }}
      />
      {draft.createIssues ? (
        <ProjectSelect
          label={t("agents.rule-creates-in")}
          valueKey={projectKey.id}
          value={draft.createsIn}
          onChange={(createsIn) => {
            onChange({ createsIn });
          }}
        />
      ) : null}
    </div>
  );
}

/** The fields of the window. */
function Fields({
  draft,
  onChange,
  searchName,
}: DraftProps & { readonly searchName: string | undefined }) {
  return (
    <div {...partOf(part.agentsRuleWindow)}>
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
      <AgentSelect draft={draft} onChange={onChange} />
      <ProjectSelect
        label={t("agents.rule-project")}
        valueKey={projectKey.id}
        value={draft.project}
        onChange={(project) => {
          onChange({ project });
        }}
      />
      <Powers draft={draft} onChange={onChange} />
      <TextField
        label={t("agents.rule-days")}
        description={t("agents.rule-days-help", {
          usual: agentRuleDaysDefault,
          most: agentRuleDaysMost,
        })}
        errorMessage={
          daysAreRight(draft.days)
            ? undefined
            : t("agents.rule-days-wrong", { most: agentRuleDaysMost })
        }
        value={draft.days}
        onChange={(days) => {
          onChange({ days });
        }}
      />
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
 * The window of a standing rule (UX §4.17, §6.3): the saved search, the agent, what it may do (only
 * propose, or change the fields of a short list, never people or the status; or create issues in a
 * project), and when it runs out (30 days, at most 90).
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

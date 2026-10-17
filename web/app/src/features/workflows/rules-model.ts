import type { components, Schema } from "@bugaboo/api-client";

/** One condition of *Only if …*: a field and what it must be like. */
export interface RuleCondition {
  /** Names the condition in the editor while it is changed; it is not sent. */
  readonly id: string;
  readonly field: string;
  readonly test: Schema<"Test">;
}

/** A field a rule can name, by the name people read. */
export interface RuleFieldChoice {
  readonly id: string;
  readonly name: string;
}

/**
 * The rules of one status change as the editor holds them: *Who can do it* (the IDs of roles and
 * groups, and two people the issue knows), *Only if …*, *Must be filled in* and *Ask for*. Naming
 * nobody means everybody who may change the issue.
 */
export interface ChangeRules {
  readonly roles: readonly string[];
  readonly groups: readonly string[];
  readonly assigned: boolean;
  readonly reportedBy: boolean;
  readonly onlyIf: readonly RuleCondition[];
  readonly mustFill: readonly string[];
  readonly askFor: readonly string[];
}

/** The rules of a status change that has none: everybody who may change the issue, nothing asked. */
export const noRules: ChangeRules = {
  roles: [],
  groups: [],
  assigned: false,
  reportedBy: false,
  onlyIf: [],
  mustFill: [],
  askFor: [],
};

/** The rules as the editor holds them, from what the API shows. */
export function rulesOf(body: Schema<"StatusRulesBody">): ChangeRules {
  return {
    roles: body.who?.roles ?? [],
    groups: body.who?.groups ?? [],
    assigned: body.who?.assigned ?? false,
    reportedBy: body.who?.reported_by ?? false,
    onlyIf: (body.only_if ?? []).map((each, at) => ({
      id: `${each.field}-${at}`,
      field: each.field,
      test: each.test,
    })),
    mustFill: body.must_fill ?? [],
    askFor: body.ask_for ?? [],
  };
}

/** The rules as *Publish* takes them. */
export function rulesBodyOf(rules: ChangeRules): components["schemas"]["StatusRulesBody"] {
  return {
    who: {
      roles: [...rules.roles],
      groups: [...rules.groups],
      assigned: rules.assigned,
      reported_by: rules.reportedBy,
    },
    only_if: rules.onlyIf.map((each) => ({ field: each.field, test: each.test })),
    must_fill: [...rules.mustFill],
    ask_for: [...rules.askFor],
  };
}

let counter = 0;

/** A condition that is new: it asks of the field that it has a value. */
export function newCondition(field: string): RuleCondition {
  counter += 1;
  return { id: `new-condition-${counter}`, field, test: "filled" };
}

/** The list with the value in it when `on`, and without it otherwise. */
export function withValue(list: readonly string[], value: string, on: boolean): readonly string[] {
  const others = list.filter((each) => each !== value);
  return on ? [...others, value] : others;
}

/** Whether two sets of rules say the same. */
export function sameRules(a: ChangeRules, b: ChangeRules): boolean {
  return JSON.stringify(rulesBodyOf(a)) === JSON.stringify(rulesBodyOf(b));
}

/** Whether the rules say anything at all: a status change with none is open to everybody who may change the issue. */
export function hasRules(rules: ChangeRules): boolean {
  return !sameRules(rules, noRules);
}

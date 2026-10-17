import { t } from "@bugaboo/messages";
import { Checkbox, partOf, part } from "@bugaboo/ui";

import { fieldNameOf } from "../../shared/field-names.ts";
import { OnlyIfRows } from "./OnlyIfRows.tsx";
import { withValue, type ChangeRules, type RuleFieldChoice } from "./rules-model.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface RulesFieldsProps {
  readonly editor: WorkflowEditor;
  readonly rules: ChangeRules;
  readonly onChange: (rules: ChangeRules) => void;
}

/** The field that is asked for in a small form can not be the description: long text has a lock of its own. */
const notAsked = "description";

/** A list of named things with a box each; ticking changes the list. */
function Boxes({
  legend,
  help,
  items,
  chosen,
  onChange,
}: {
  readonly legend: string;
  readonly help?: string;
  readonly items: readonly { readonly id: string; readonly name: string }[];
  readonly chosen: readonly string[];
  readonly onChange: (chosen: readonly string[]) => void;
}) {
  return (
    <fieldset {...partOf(part.workflowRules)}>
      <legend>{legend}</legend>
      {help === undefined ? null : <p {...partOf(part.muted)}>{help}</p>}
      {items.map((item) => (
        <Checkbox
          key={item.id}
          label={item.name}
          isSelected={chosen.includes(item.id)}
          onChange={(on) => {
            onChange(withValue(chosen, item.id, on));
          }}
        />
      ))}
    </fieldset>
  );
}

/** *Who can do it*: roles and groups, and the person assigned and the person who reported the issue. */
function WhoCan({ editor, rules, onChange }: RulesFieldsProps) {
  return (
    <fieldset {...partOf(part.workflowRules)}>
      <legend>{t("workflows.who")}</legend>
      <p {...partOf(part.muted)}>{t("workflows.who-help")}</p>
      <Boxes
        legend={t("workflows.who-roles")}
        items={editor.roles}
        chosen={rules.roles}
        onChange={(roles) => {
          onChange({ ...rules, roles });
        }}
      />
      {editor.groups.length === 0 ? null : (
        <Boxes
          legend={t("workflows.who-groups")}
          items={editor.groups}
          chosen={rules.groups}
          onChange={(groups) => {
            onChange({ ...rules, groups });
          }}
        />
      )}
      <Checkbox
        label={t("workflows.who-assigned")}
        isSelected={rules.assigned}
        onChange={(assigned) => {
          onChange({ ...rules, assigned });
        }}
      />
      <Checkbox
        label={t("workflows.who-reporter")}
        isSelected={rules.reportedBy}
        onChange={(reportedBy) => {
          onChange({ ...rules, reportedBy });
        }}
      />
    </fieldset>
  );
}

/** The rules of a status change in plain words, each in a group of its own. */
export function RulesFields(props: RulesFieldsProps) {
  const { editor, rules, onChange } = props;
  const named: readonly RuleFieldChoice[] = editor.fields.map((each) => ({
    id: each.field,
    name: fieldNameOf({ code: each.field, name: each.name }),
  }));
  return (
    <>
      <WhoCan {...props} />
      <OnlyIfRows fields={named} rules={rules} onChange={onChange} />
      <Boxes
        legend={t("workflows.must-fill")}
        help={t("workflows.must-fill-help")}
        items={named}
        chosen={rules.mustFill}
        onChange={(mustFill) => {
          onChange({ ...rules, mustFill });
        }}
      />
      <Boxes
        legend={t("workflows.ask-for")}
        help={t("workflows.ask-for-help")}
        items={named.filter((each) => each.id !== notAsked)}
        chosen={rules.askFor}
        onChange={(askFor) => {
          onChange({ ...rules, askFor });
        }}
      />
    </>
  );
}

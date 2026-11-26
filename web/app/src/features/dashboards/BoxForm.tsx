import { t } from "@bugaboo/messages";
import { Form, part, partOf, SelectField, SubmitButton, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { notGroupable } from "../../shared/list-columns.ts";
import { useSearchFields } from "../search/index.ts";
import {
  askedOf,
  draftOf,
  hasShape,
  hasSpan,
  noField,
  readsSearch,
  type Draft,
} from "./box-draft.ts";
import { boxKind, kindName, timeSpan, type AskedBox } from "./dashboards.ts";

/** The kinds of box in the order *Add box* offers them (ADR-0050, decision 4). */
const kinds = Object.values(boxKind);

/** The field a chart groups its bars by, or none for one number: any field a list can be grouped by. */
function GroupByField({
  draft,
  onChange: change,
}: {
  readonly draft: Draft;
  readonly onChange: (changed: Partial<Draft>) => void;
}) {
  const fields = useSearchFields();
  const names = fields.data?.names ?? {};
  const groupable = Object.keys(names).filter((word) => !notGroupable.has(word));
  return (
    <SelectField
      label={t("dashboards.box-by")}
      choices={[
        { value: noField, label: t("dashboards.box-by-none") },
        ...groupable.map((word) => ({ value: word, label: names[word] ?? word })),
      ]}
      value={draft.by}
      onChange={(by) => {
        change({ by });
      }}
    />
  );
}

/** The options of the kind of a box: whose time it adds up, the search it reads, the field a chart groups by. */
function OptionFields({
  draft,
  start,
  onChange: change,
}: {
  readonly draft: Draft;
  readonly start: AskedBox;
  readonly onChange: (changed: Partial<Draft>) => void;
}) {
  return (
    <>
      {hasSpan(draft.kind) ? (
        <SelectField
          label={t("dashboards.box-span")}
          choices={[
            { value: timeSpan.search, label: t("dashboards.span.search") },
            { value: timeSpan.myWeek, label: t("dashboards.span.my-week") },
          ]}
          value={draft.span}
          onChange={(span) => {
            change({ span });
          }}
        />
      ) : null}
      {readsSearch(draft.kind, draft.span) && (start.saved_search ?? null) === null ? (
        <TextField
          label={t("dashboards.box-search")}
          description={t("dashboards.box-search-hint")}
          value={draft.text}
          onChange={(text) => {
            change({ text });
          }}
        />
      ) : null}
      {hasShape(draft.kind) ? <GroupByField draft={draft} onChange={change} /> : null}
    </>
  );
}

export interface BoxFormProps {
  /** The box as it is now (*Change box*), or as a box of your own starts. */
  readonly start: AskedBox;
  /** The kind can be chosen: a box of your own; *Change box* keeps the kind. */
  readonly chooseKind: boolean;
  /** What the button says: *Add the box*, *Save*. */
  readonly submit: string;
  readonly pending: boolean;
  readonly onSubmit: (box: AskedBox) => void;
}

/**
 * The form of a box (UX §4.10): its kind (for a box of your own), its title, the search it reads, and the options of
 * its kind: the field a chart groups by (none for one number), whose time *Time totals* adds up. There is no option that
 * sorts by size or ranks people (ADR-0050, decision 5).
 */
export function BoxForm({ start, chooseKind, submit, pending, onSubmit }: BoxFormProps) {
  const [draft, setDraft] = useState<Draft>(() => draftOf(start));
  const change = (changed: Partial<Draft>): void => {
    setDraft({ ...draft, ...changed });
  };
  return (
    <Form
      onSubmit={() => {
        onSubmit(askedOf(start, draft));
      }}
    >
      <div {...partOf(part.boxForm)}>
        {chooseKind ? (
          <SelectField
            label={t("dashboards.box-kind")}
            choices={kinds.map((kind) => ({ value: kind, label: t(kindName[kind]) }))}
            value={draft.kind}
            onChange={(kind) => {
              change({ kind });
            }}
          />
        ) : null}
        <TextField
          label={t("dashboards.box-title")}
          requiredMessage={t("error.field.required")}
          value={draft.title}
          onChange={(title) => {
            change({ title });
          }}
        />
        <OptionFields draft={draft} start={start} onChange={change} />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={pending}>{submit}</SubmitButton>
        </div>
      </div>
    </Form>
  );
}

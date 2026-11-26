import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  part,
  partOf,
  SelectField,
  SubmitButton,
  TextField,
  Window,
} from "@bugaboo/ui";
import { useId } from "react";

import { windowLabels } from "../../shared/window-labels.ts";
import { columnName } from "./card-moves.ts";
import { noRows, statusChoice, useCustomize, type CustomizeInput } from "./use-customize.ts";

/** What the window of *Customize* offers: the fields the columns and the rows can come from, by their words. */
export interface CustomizeChoices {
  /** The words of the fields that hold one choice (*Severity*), with their names; *Status* comes before them. */
  readonly columns: readonly { readonly value: string; readonly label: string }[];
  /** The words of the fields the rows can come from, with their names. */
  readonly rows: readonly { readonly value: string; readonly label: string }[];
}

export interface CustomizeWindowProps extends CustomizeInput {
  readonly choices: CustomizeChoices;
}

/** What the boxes of the card limits are given. */
interface LimitsProps {
  /** The columns the board has: the limits belong to them. */
  readonly columns: CustomizeInput["columns"];
  /** The columns are the saved ones: otherwise the boxes wait for them. */
  readonly same: boolean;
  readonly textOf: (column: CustomizeInput["columns"][number]) => string;
  readonly problems: readonly (string | undefined)[];
  readonly onText: (column: CustomizeInput["columns"][number], value: string) => void;
}

/** A box for the card limit of each column, or the note that they wait for the new columns. */
function Limits({ columns, same, textOf, problems, onText }: LimitsProps) {
  if (!same) {
    return <p {...partOf(part.muted)}>{t("board.limits-later")}</p>;
  }
  return (
    <div {...partOf(part.boardCustomizeLimits)}>
      {columns.map((column, at) => (
        <TextField
          key={column.key ?? ""}
          label={t("board.card-limit-of", { column: columnName(column) })}
          value={textOf(column)}
          errorMessage={problems[at]}
          onChange={(value) => {
            onText(column, value);
          }}
        />
      ))}
    </div>
  );
}

/** *Cancel* and *Save the board*, the main one at the end. */
function Footer({
  formId,
  pending,
  onClose,
}: {
  readonly formId: string;
  readonly pending: boolean;
  readonly onClose: () => void;
}) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <SubmitButton form={formId} isPending={pending}>
        {t("board.save")}
      </SubmitButton>
    </div>
  );
}

/**
 * *Customize* of a board (UX §4.6): which field the columns come from, which field the rows come
 * from, and a card limit for each column (empty means none). Only the maker of the saved search sees
 * it.
 */
export function CustomizeWindow({ choices, ...input }: CustomizeWindowProps) {
  const formId = useId();
  const form = useCustomize(input);
  const { onClose } = input;
  return (
    <Window
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("board.customize-title")}
      labels={windowLabels()}
      footer={<Footer formId={formId} pending={form.pending} onClose={onClose} />}
    >
      <Form id={formId} onSubmit={form.send}>
        <div {...partOf(part.boardCustomize)}>
          <SelectField
            label={t("board.columns-from")}
            choices={[{ value: statusChoice, label: t("issue-field.status") }, ...choices.columns]}
            value={form.by}
            onChange={form.setBy}
          />
          <SelectField
            label={t("board.rows")}
            choices={[{ value: noRows, label: t("board.no-rows") }, ...choices.rows]}
            value={form.rows}
            onChange={form.setRows}
          />
          <p {...partOf(part.muted)}>{t("board.card-limits-help")}</p>
          <Limits
            columns={input.columns}
            same={form.same}
            textOf={form.textOf}
            problems={form.problems}
            onText={form.setText}
          />
          {form.refused === undefined ? null : (
            <Alert tone={alertTone.danger}>{form.refused}</Alert>
          )}
        </div>
      </Form>
    </Window>
  );
}

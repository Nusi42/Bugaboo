import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  defaultDeviceThemes,
  Form,
  partOf,
  part,
  SelectField,
  SubmitButton,
  TextField,
  themeIds,
  type ThemeId,
} from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { themeLabels } from "../../shared/theme-labels.ts";
import { useCreateTheme } from "./themes.ts";
import { themeEditorAddress } from "./addresses.ts";

/** The built-in theme a new theme starts from until the person chooses another: the light one that starts. */
const FIRST_CHOICE: ThemeId = defaultDeviceThemes.light;

/**
 * *Create a theme* (UX §5.6, M8-Q11): a name and the built-in theme it starts from (*Daylight*,
 * *Paper*, *Dusk* or *Midnight*), whose kind, light or dark, it takes. It opens in the editor.
 */
export function CreateTheme({ kind }: { readonly kind: Schema<"ThemeKind"> }) {
  const [name, setName] = useState("");
  const [startsFrom, setStartsFrom] = useState<ThemeId>(FIRST_CHOICE);
  const navigate = useNavigate();
  const create = useCreateTheme();
  return (
    <Form
      onSubmit={() => {
        create.mutate(
          { body: { name: name.trim(), kind, extends: startsFrom } },
          {
            onSuccess: (made: Schema<"ThemeBody">) => {
              void navigate({ to: themeEditorAddress(made.kind, made.id) });
            },
          },
        );
      }}
    >
      <TextField
        label={t("theme-editor.create-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <SelectField
        label={t("theme-editor.create-from")}
        value={startsFrom}
        choices={themeIds.map((id) => ({ value: id, label: themeLabels[id] }))}
        onChange={setStartsFrom}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("theme-editor.create")}</SubmitButton>
      </div>
    </Form>
  );
}

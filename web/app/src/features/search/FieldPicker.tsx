import { t } from "@bugaboo/messages";
import { Button, buttonLook, PickerRow, SearchPicker } from "@bugaboo/ui";
import type { Field, ReadableFields } from "@bugaboo/wasm-search";
import { useState } from "react";

import { fieldLabel } from "./filter-text.ts";

export interface FieldPickerProps {
  readonly fields: ReadableFields;
  readonly onPick: (field: Field) => void;
}

/** The fields whose name or word has the words typed in it. */
function matching(fields: ReadableFields, words: string): readonly Field[] {
  const wanted = words.trim().toLowerCase();
  return fields.fields.filter(
    (field) =>
      wanted === "" ||
      fieldLabel(field.word).toLowerCase().includes(wanted) ||
      field.word.toLowerCase().includes(wanted),
  );
}

/** The fields the person may read, the ones used most first (the order the server gives), with a box to find the rest. */
export function FieldPicker({ fields, onPick }: FieldPickerProps) {
  const [words, setWords] = useState("");
  const found = matching(fields, words);
  return (
    <SearchPicker
      legend={t("search.bar.fields-legend")}
      searchLabel={t("search.bar.fields-search")}
      clearLabel={t("search.bar.fields-clear")}
      listLabel={t("search.bar.fields-list")}
      query={words}
      onQuery={setWords}
      failed={undefined}
      nothing={found.length === 0 ? t("search.bar.fields-none") : undefined}
      more={undefined}
    >
      {found.map((field) => (
        <PickerRow key={field.word} label={fieldLabel(field.word)}>
          <Button
            look={buttonLook.quiet}
            aria-label={t("search.bar.filter-by", { field: fieldLabel(field.word) })}
            onPress={() => {
              onPick(field);
            }}
          >
            {t("search.bar.filter-by-short")}
          </Button>
        </PickerRow>
      ))}
    </SearchPicker>
  );
}

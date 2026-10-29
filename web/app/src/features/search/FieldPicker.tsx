import { t } from "@bugaboo/messages";
import { PickerChoice, SearchPicker } from "@bugaboo/ui";
import type { Field, ReadableFields } from "@bugaboo/wasm-search";
import { useState } from "react";

import { useFieldName, type FieldNameOf } from "./field-name-context.ts";

export interface FieldPickerProps {
  readonly fields: ReadableFields;
  readonly onPick: (field: Field) => void;
}

/** The fields whose name or word has the words typed in it. */
function matching(fields: ReadableFields, words: string, nameOf: FieldNameOf): readonly Field[] {
  const wanted = words.trim().toLowerCase();
  return fields.fields.filter(
    (field) =>
      wanted === "" ||
      nameOf(field.word).toLowerCase().includes(wanted) ||
      field.word.toLowerCase().includes(wanted),
  );
}

/** The fields the person may read, the ones used most first (the order the server gives), with a box to find the rest. */
export function FieldPicker({ fields, onPick }: FieldPickerProps) {
  const [words, setWords] = useState("");
  const nameOf = useFieldName();
  const found = matching(fields, words, nameOf);
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
        <PickerChoice
          key={field.word}
          label={nameOf(field.word)}
          accessibleName={t("search.bar.filter-by", { field: nameOf(field.word) })}
          onPick={() => {
            onPick(field);
          }}
        />
      ))}
    </SearchPicker>
  );
}

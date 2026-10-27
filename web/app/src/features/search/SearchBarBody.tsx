import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf } from "@bugaboo/ui";
import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import type { ReactNode } from "react";

import { AdvancedSearch } from "./AdvancedSearch.tsx";
import { FieldNames } from "./field-name-context.ts";
import { HelpButton } from "./HelpButton.tsx";
import { SimpleSearch } from "./SimpleSearch.tsx";
import { barMode, type SearchBarState } from "./use-search-bar.ts";

export interface SearchBarBodyProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** How each readable field is called, by its word. */
  readonly names: Readonly<Record<string, string>>;
  readonly bar: SearchBarState;
  readonly actions?: ReactNode;
}

/** The bar itself: the mode, the filters or the box, and the buttons at the end. */
export function SearchBarBody({ language, fields, names, bar, actions }: SearchBarBodyProps) {
  const simple = bar.mode === barMode.simple && bar.search !== null;
  return (
    <FieldNames value={(word) => names[word] ?? word}>
      <div {...partOf(part.searchBarHead)}>
        {simple && bar.search !== null ? (
          <SimpleSearch language={language} fields={fields} bar={bar} search={bar.search} />
        ) : null}
        <div {...partOf(part.searchBarActions)}>
          <Button
            look={buttonLook.quiet}
            isDisabled={!simple && !bar.canBeSimple}
            onPress={() => {
              bar.switchTo(simple ? barMode.advanced : barMode.simple);
            }}
          >
            {simple ? t("search.bar.to-advanced") : t("search.bar.to-simple")}
          </Button>
          <HelpButton language={language} fields={fields} bar={bar} />
          {actions}
        </div>
      </div>
      {simple ? null : <AdvancedSearch language={language} fields={fields} bar={bar} />}
    </FieldNames>
  );
}

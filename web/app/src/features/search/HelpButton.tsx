import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { HelpSheet } from "./HelpSheet.tsx";
import { withExample } from "../../shared/search-tree.ts";
import type { SearchBarState } from "./use-search-bar.ts";

export interface HelpButtonProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly bar: SearchBarState;
}

/** *Help* opens the sheet of the search language (UX design section 4.3); an example goes into the search with one click. */
export function HelpButton({ language, fields, bar }: HelpButtonProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("search.help")}
      </Button>
      <HelpSheet
        language={language}
        fields={fields}
        isOpen={open}
        onClose={() => {
          setOpen(false);
        }}
        onExample={(text) => {
          const example = language.parse(text).search;
          if (example !== null && bar.search !== null) {
            bar.change(withExample(bar.search, example));
          }
        }}
      />
    </>
  );
}

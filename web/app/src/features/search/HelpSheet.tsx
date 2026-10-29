import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, overlayEdge, part, partOf } from "@bugaboo/ui";
import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";

import { useFieldName } from "./field-name-context.ts";
import { helpGroups, type HelpGroup } from "./help-examples.ts";

export interface HelpSheetProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** Puts an example into the search: its text. */
  readonly onExample: (text: string) => void;
}

/** One group of the sheet with the examples the checker accepts for this person. */
function HelpGroupView({
  group,
  language,
  fields,
  onExample,
}: Pick<HelpSheetProps, "language" | "fields" | "onExample"> & { readonly group: HelpGroup }) {
  const examples = group.examples.filter(
    (text) => language.check(text, fields).mistakes.length === 0,
  );
  return examples.length === 0 ? null : (
    <section {...partOf(part.searchHelpGroup)}>
      <h3 {...partOf(part.searchHelpTitle)}>{t(group.heading)}</h3>
      <p {...partOf(part.muted)}>{t(group.text)}</p>
      {examples.map((text) => (
        <div key={text} {...partOf(part.searchExample)}>
          <code>{text}</code>
          <Button
            look={buttonLook.quiet}
            aria-label={t("search.help-use", { example: text })}
            onPress={() => {
              onExample(text);
            }}
          >
            {t("search.help-use-short")}
          </Button>
        </div>
      ))}
    </section>
  );
}

/**
 * The help sheet (UX design section 4.3): the words of the search language in groups, each with
 * examples that go into the search with one click, and the fields the person may use with their
 * words. An example that the checker does not accept for this person (it names a field they may
 * not read) is not shown.
 */
export function HelpSheet({ language, fields, isOpen, onClose, onExample }: HelpSheetProps) {
  const nameOf = useFieldName();
  return (
    <Overlay
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={t("search.help-title")}
      from={overlayEdge.end}
    >
      <div {...partOf(part.searchPanel)}>
        <div {...partOf(part.searchPanelHead)}>
          <h2 {...partOf(part.confirmTitle)}>{t("search.help-title")}</h2>
          <Button onPress={onClose}>{t("common.close")}</Button>
        </div>
        <div {...partOf(part.searchHelp)}>
          {helpGroups.map((group) => (
            <HelpGroupView
              key={group.heading}
              group={group}
              language={language}
              fields={fields}
              onExample={onExample}
            />
          ))}
          <section {...partOf(part.searchHelpGroup)}>
            <h3 {...partOf(part.searchHelpTitle)}>{t("search.help-your-fields")}</h3>
            <p {...partOf(part.muted)}>{t("search.help-your-fields-text")}</p>
            <ul {...partOf(part.searchList)}>
              {fields.fields.map((field) => (
                <li key={field.word} {...partOf(part.searchListItem)}>
                  <span>{nameOf(field.word)}</span>
                  <code>{field.word}</code>
                </li>
              ))}
            </ul>
          </section>
        </div>
      </div>
    </Overlay>
  );
}

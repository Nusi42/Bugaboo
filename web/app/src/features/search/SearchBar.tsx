import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  Overlay,
  overlayEdge,
  Skeleton,
  useLayout,
  part,
  partOf,
} from "@bugaboo/ui";
import { useEffect, useState, type ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";

import { useSearchFields } from "./search-fields.ts";
import { useSearchLanguage } from "./search-language.ts";
import { filtersOf } from "../../shared/search-tree.ts";
import { SearchBarBody } from "./SearchBarBody.tsx";
import { useSearchKey } from "./use-search-key.ts";
import { useSearchBar, type SearchBarInput } from "./use-search-bar.ts";

export interface SearchBarProps {
  /** The canonical text the address holds. */
  readonly text: string;
  /** Writes the canonical text of the search into the address. */
  readonly onText: SearchBarInput["onText"];
  /** The ID of the project the search looks in; none looks in every project the person sees. */
  readonly within?: string;
  /** Called when the text in the box has a mistake (`true`) or has none (`false`). */
  readonly onStale?: (stale: boolean) => void;
  /** Buttons for what is done with the search (*Save search*, *Share*, *Help*), at the end. */
  readonly actions?: ReactNode;
}

/**
 * The search bar above every list (QRY-01 to QRY-03, UX design section 4.3): Simple search with
 * filter buttons, or Advanced search as text. Both edit the same tree, so switching never loses
 * anything. On a phone it is one button, *Filters (n)*, that opens the same in a sheet.
 */
export function SearchBar({ text, onText, within, onStale, actions }: SearchBarProps) {
  const language = useSearchLanguage();
  const fields = useSearchFields(within);
  if (fields.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(fields.error)}</Alert>;
  }
  if (language === undefined || fields.data === undefined) {
    return <Skeleton lines={1} />;
  }
  return (
    <LoadedBar
      language={language}
      fields={fields.data.fields}
      names={fields.data.names}
      text={text}
      onText={onText}
      onStale={onStale}
      actions={actions}
    />
  );
}

type LoadedProps = Omit<SearchBarProps, "within" | "onStale"> & {
  readonly onStale: ((stale: boolean) => void) | undefined;
  readonly names: Readonly<Record<string, string>>;
} & Required<Pick<Parameters<typeof useSearchBar>[0], "language" | "fields">>;

function LoadedBar({ language, fields, names, text, onText, onStale, actions }: LoadedProps) {
  const bar = useSearchBar({ language, fields, text, onText });
  const stale = bar.answer.mistakes.length > 0;
  useEffect(() => {
    onStale?.(stale);
  }, [onStale, stale]);
  const layout = useLayout();
  const [open, setOpen] = useState(false);
  const container = useSearchKey(() => {
    setOpen(true);
  });
  const body = (
    <SearchBarBody language={language} fields={fields} names={names} bar={bar} actions={actions} />
  );
  if (layout !== "phone") {
    return (
      <div {...partOf(part.searchBar)} ref={container}>
        {body}
      </div>
    );
  }
  const count = bar.search === null ? 0 : filtersOf(bar.search).length;
  return (
    <div {...partOf(part.searchBar)}>
      <div {...partOf(part.searchBarHead)}>
        <Button
          onPress={() => {
            setOpen(true);
          }}
        >
          {bar.search === null ? t("search.bar.filters") : t("search.bar.filters-count", { count })}
        </Button>
      </div>
      <Overlay
        isOpen={open}
        onOpenChange={setOpen}
        label={t("search.bar.filters")}
        from={overlayEdge.bottom}
      >
        <div {...partOf(part.searchPanel)}>
          <div {...partOf(part.searchBar)}>{body}</div>
        </div>
      </Overlay>
    </div>
  );
}

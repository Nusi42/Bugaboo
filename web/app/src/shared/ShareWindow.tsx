import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Checkbox,
  part,
  partOf,
  ShowMoreOf,
  Window,
  type PagedList,
} from "@bugaboo/ui";
import type { ReactNode } from "react";

import { messageOf } from "./errors.ts";
import { itemsOf } from "./paged.ts";
import { shareKind, type ShareTarget } from "./share.ts";
import { windowLabels } from "./window-labels.ts";

/** A group or a project the person may share with. */
export type ShareChoice = Schema<"ShareChoiceItem">;

/** The groups or the projects the person may share with, a page at a time. */
export type ShareChoices = PagedList & {
  readonly data?:
    | { readonly pages: readonly { readonly items: readonly ShareChoice[] }[] }
    | undefined;
  readonly isPending: boolean;
  readonly isError: boolean;
  readonly error: unknown;
};

/** Shares with a group or a project, or stops sharing. */
export interface ShareChange {
  readonly mutate: (change: { readonly on: boolean; readonly target: ShareTarget }) => void;
  readonly isError: boolean;
  readonly error: unknown;
}

/**
 * What is shared and how (ADR-0034; ADR-0050, decision 2): whom it is shared with now, the choices and the change of a
 * share, and what a row of a choice shows beside its tick box.
 */
export interface Sharing {
  readonly sharedWith: readonly Schema<"ShareRef">[];
  /** The groups and the projects the person may share with, read while the window is open. */
  readonly choices: Readonly<Record<Schema<"ShareKind">, ShareChoices>>;
  readonly change: ShareChange;
  readonly renderExtra?: (
    kind: Schema<"ShareKind">,
    item: ShareChoice,
    shared: boolean,
  ) => ReactNode;
}

/** One group or project: its tick box, and what the shared thing shows beside it. */
function ShareRow({
  sharing,
  kind,
  item,
  onToggle,
}: {
  readonly sharing: Sharing;
  readonly kind: Schema<"ShareKind">;
  readonly item: ShareChoice;
  readonly onToggle: (on: boolean) => void;
}) {
  const shared = sharing.sharedWith.some((share) => share.kind === kind && share.id === item.id);
  return (
    <li {...partOf(part.shareRow)}>
      <div {...partOf(part.shareChoice)}>
        <Checkbox label={item.name} isSelected={shared} onChange={onToggle} />
        {sharing.renderExtra?.(kind, item, shared)}
      </div>
    </li>
  );
}

/** The words of the list of one kind. */
const kindWords = {
  group: {
    legend: "search.share-groups",
    none: "search.share-none-groups",
    more: "search.share-more-groups",
  },
  project: {
    legend: "search.share-projects",
    none: "search.share-none-projects",
    more: "search.share-more-projects",
  },
} as const satisfies Record<Schema<"ShareKind">, Record<string, string>>;

/** The groups or the projects the person may share with, each a box to tick; a tick shares at once. */
function Choices({
  sharing,
  kind,
}: {
  readonly sharing: Sharing;
  readonly kind: Schema<"ShareKind">;
}) {
  const choices = sharing.choices[kind];
  const { change } = sharing;
  const items = itemsOf(choices.data?.pages);
  const words = kindWords[kind];
  return (
    <fieldset {...partOf(part.searchHelpGroup)}>
      <legend {...partOf(part.searchHelpTitle)}>{t(words.legend)}</legend>
      {choices.isError ? <Alert tone={alertTone.danger}>{messageOf(choices.error)}</Alert> : null}
      {items.length === 0 && !choices.isPending && !choices.isError ? (
        <p {...partOf(part.muted)}>{t(words.none)}</p>
      ) : null}
      <ul {...partOf(part.searchList)}>
        {items.map((item) => (
          <ShareRow
            key={item.id}
            sharing={sharing}
            kind={kind}
            item={item}
            onToggle={(on) => {
              change.mutate({ on, target: { kind, id: item.id } });
            }}
          />
        ))}
      </ul>
      <ShowMoreOf label={t(words.more)} list={choices} />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </fieldset>
  );
}

export interface ShareWindowProps {
  /** The title of the window: *Share Open bugs*. */
  readonly title: string;
  /** The words under the lists: that each person still sees only what their own rights allow. */
  readonly note: string;
  readonly sharing: Sharing;
  readonly isOpen: boolean;
  readonly onClose: () => void;
}

/**
 * The *Share* window of a saved search and of a dashboard (ADR-0034; ADR-0050, decision 2): a tick shares with a group
 * or a project, and only those the person may share with are listed. Under them, the words that each person still sees
 * only what their own rights allow. The one window of sharing, so both share the same way.
 */
export function ShareWindow({ title, note, sharing, isOpen, onClose }: ShareWindowProps) {
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={title}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.primary} onPress={onClose}>
            {t("search.share-done")}
          </Button>
        </div>
      }
    >
      <div {...partOf(part.shareBody)}>
        <Choices sharing={sharing} kind={shareKind.group} />
        <Choices sharing={sharing} kind={shareKind.project} />
        <p {...partOf(part.muted)}>{note}</p>
      </div>
    </Window>
  );
}

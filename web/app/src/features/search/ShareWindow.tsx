import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Checkbox, part, partOf, Window } from "@bugaboo/ui";
import type { Schema } from "@bugaboo/api-client";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import { ProjectListToggle } from "./ProjectListToggle.tsx";
import { shareKind, useChangeShare, useShareChoices, type SavedSearch } from "./saved-searches.ts";

export interface ShareWindowProps {
  readonly search: SavedSearch;
  readonly isOpen: boolean;
  readonly onClose: () => void;
}

/** Whether the saved search is shared with this group or project already. */
function isSharedWith(search: SavedSearch, kind: Schema<"ShareKind">, id: string): boolean {
  return search.shared_with.some((share) => share.kind === kind && share.id === id);
}

/** *Show more* under a list of choices that has another page. */
function MoreChoices({
  hasMore,
  isPending,
  onMore,
}: {
  readonly hasMore: boolean;
  readonly isPending: boolean;
  readonly onMore: () => void;
}) {
  return hasMore ? (
    <Button isPending={isPending} onPress={onMore}>
      {t("search.share-more")}
    </Button>
  ) : null;
}

/** One group or project: its tick box, and for a project of this search its quiet *Use as the list* button. */
function ShareRow({
  search,
  kind,
  item,
  onToggle,
}: {
  readonly search: SavedSearch;
  readonly kind: Schema<"ShareKind">;
  readonly item: {
    readonly id: string;
    readonly name: string;
    readonly short_name?: string | null;
  };
  readonly onToggle: (on: boolean) => void;
}) {
  return (
    <li {...partOf(part.shareRow)}>
      <div {...partOf(part.shareChoice)}>
        <Checkbox
          label={item.name}
          isSelected={isSharedWith(search, kind, item.id)}
          onChange={onToggle}
        />
        {kind === shareKind.project &&
        isSharedWith(search, kind, item.id) &&
        item.short_name !== undefined &&
        item.short_name !== null ? (
          <ProjectListToggle search={search} project={item.short_name} name={item.name} />
        ) : null}
      </div>
    </li>
  );
}

/** The groups or the projects the person may share with, each a box to tick; a tick shares at once. */
function Choices({
  search,
  kind,
  legend,
  none,
}: {
  readonly search: SavedSearch;
  readonly kind: Schema<"ShareKind">;
  readonly legend: string;
  readonly none: string;
}) {
  const choices = useShareChoices(kind);
  const change = useChangeShare(search.id);
  const items = itemsOf(choices.data?.pages);
  return (
    <fieldset {...partOf(part.searchHelpGroup)}>
      <legend {...partOf(part.searchHelpTitle)}>{legend}</legend>
      {choices.isError ? <Alert tone={alertTone.danger}>{messageOf(choices.error)}</Alert> : null}
      {items.length === 0 && !choices.isPending && !choices.isError ? (
        <p {...partOf(part.muted)}>{none}</p>
      ) : null}
      <ul {...partOf(part.searchList)}>
        {items.map((item) => (
          <ShareRow
            key={item.id}
            search={search}
            kind={kind}
            item={item}
            onToggle={(on) => {
              change.mutate({ on, target: { kind, id: item.id } });
            }}
          />
        ))}
      </ul>
      <MoreChoices
        hasMore={choices.hasNextPage}
        isPending={choices.isFetchingNextPage}
        onMore={() => {
          void choices.fetchNextPage();
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </fieldset>
  );
}

/**
 * The *Share* window (ADR-0034, UX design section 4.3): a tick shares the saved search with a group
 * or a project, and only those the person may share with are listed. Under them, the words that
 * a receiver still sees only what their own rights allow.
 */
export function ShareWindow({ search, isOpen, onClose }: ShareWindowProps) {
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("search.share-title", { name: search.name })}
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
        <Choices
          search={search}
          kind={shareKind.group}
          legend={t("search.share-groups")}
          none={t("search.share-none-groups")}
        />
        <Choices
          search={search}
          kind={shareKind.project}
          legend={t("search.share-projects")}
          none={t("search.share-none-projects")}
        />
        <p {...partOf(part.muted)}>{t("search.share-note")}</p>
      </div>
    </Window>
  );
}

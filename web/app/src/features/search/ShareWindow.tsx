import { t } from "@bugaboo/messages";

import { shareKind } from "../../shared/share.ts";
import { ShareWindow as SharingWindow } from "../../shared/ShareWindow.tsx";
import { ProjectListToggle } from "./ProjectListToggle.tsx";
import { useChangeShare, useShareChoices, type SavedSearch } from "./saved-searches.ts";

export interface ShareWindowProps {
  readonly search: SavedSearch;
  readonly isOpen: boolean;
  readonly onClose: () => void;
}

/**
 * The *Share* window of a saved search (ADR-0034, UX design section 4.3): the one window of sharing, with the quiet
 * *Use as the list* button beside a project the search is shared with.
 */
export function ShareWindow({ search, isOpen, onClose }: ShareWindowProps) {
  const groups = useShareChoices(shareKind.group, isOpen);
  const projects = useShareChoices(shareKind.project, isOpen);
  const change = useChangeShare(search.id);
  return (
    <SharingWindow
      title={t("search.share-title", { name: search.name })}
      note={t("search.share-note")}
      isOpen={isOpen}
      onClose={onClose}
      sharing={{
        sharedWith: search.shared_with,
        choices: { group: groups, project: projects },
        change,
        renderExtra: (kind, item, shared) =>
          kind === shareKind.project &&
          shared &&
          item.short_name !== undefined &&
          item.short_name !== null ? (
            <ProjectListToggle search={search} project={item.short_name} name={item.name} />
          ) : null,
      }}
    />
  );
}

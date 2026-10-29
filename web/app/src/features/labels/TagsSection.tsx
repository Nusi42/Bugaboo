import { t } from "@bugaboo/messages";
import { Card, partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditTag } from "./EditTag.tsx";
import { tagLinesOf, useTags, type TagLine } from "./labels.ts";
import type { OpenProps } from "./OpenProps.ts";

/** The row of a tag: the projects it is for beside its name. */
function rowOf(tag: TagLine, tags: readonly TagLine[]): ListRow {
  return {
    key: tag.id,
    name: tag.name,
    details: (
      <span {...partOf(part.muted)}>
        {tag.wholeOrganization
          ? t("labels.tag-everywhere")
          : t("labels.tag-in-projects", { count: tag.projects.length })}
      </span>
    ),
    editor: <EditTag key={tag.id} tag={tag} others={tags} />,
  };
}

/** The tags, a page at a time, as a list that opens next to it. */
export function TagsSection({ open, onOpen, onClose }: OpenProps) {
  const tags = useTags();
  const lines = tagLinesOf(tags.data?.pages);
  return (
    <Card heading={t("labels.tags-title")}>
      <ListEditor
        rows={lines.map((tag) => rowOf(tag, lines))}
        searchLabel={t("labels.tags-search")}
        nothing={t("labels.tags-nothing")}
        none={t("labels.tags-none-yet")}
        openKey={open}
        onOpen={onOpen}
        onClose={onClose}
      />
      {<ShowMoreOf label={t("labels.tags-more")} list={tags} />}
    </Card>
  );
}

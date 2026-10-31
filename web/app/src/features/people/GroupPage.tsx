import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotFoundPage, QueryGate, useItemTrail } from "../shell/index.ts";
import { GroupActions } from "./DeleteGroup.tsx";
import { GroupMembers } from "./GroupMembers.tsx";
import { useGroup } from "./groups.ts";
import { HeldRoles } from "./PersonSections.tsx";
import { GroupNameCard } from "./RenameGroup.tsx";

/**
 * The page of one group (SEC-02), opened like the page of a person and of a role: its name and what it is used for,
 * the people in it and the picker that puts more in, its roles, and *Delete the group*, with *All groups* back to the
 * list.
 */
export function GroupPage({ id }: { readonly id: string }) {
  const found = useGroup(id);
  const detail = found.data;
  usePageTitle(detail?.group.name ?? t("nav.groups"));
  useItemTrail(paths.groups, detail?.group.name);
  return (
    <QueryGate error={found.error} data={detail} notFound={<NotFoundPage />}>
      {(page) => (
        <div {...partOf(part.stack)}>
          <GroupNameCard group={page.group} used={page.used_by} />
          <Card heading={t("groups.members")}>
            <GroupMembers group={page.group} />
          </Card>
          <HeldRoles
            roles={page.roles}
            none={t("groups.roles-none", { name: page.group.name })}
            cut={page.truncated ? t("groups.roles-cut") : undefined}
          />
          <GroupActions group={page.group} members={page.used_by.members} />
        </div>
      )}
    </QueryGate>
  );
}

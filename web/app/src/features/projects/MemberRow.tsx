import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { percentText } from "../../shared/number.ts";
import { namedPerson, personName } from "../../shared/person.ts";
import { NumberBox } from "../fields/index.ts";
import type { MemberShare, ShareChanges } from "./shares.ts";

/**
 * The mark of a person whose shares add up to more than 100 %: *Over 100 % in all*, with the other
 * projects the reader may name, and that there are others when there are (never which).
 */
function OverMark({ member }: { readonly member: MemberShare }) {
  if (member.over_100 !== true) {
    return null;
  }
  const named = (member.others ?? []).flatMap((project) => project.name ?? []);
  const where = member.others_hidden ? [...named, t("project-members.others-hidden")] : named;
  return (
    <>
      <span {...partOf(part.muted, { variants: ["warning"] })}>{t("project-members.over")}</span>
      {where.length === 0 ? null : (
        <span {...partOf(part.muted)}>
          {t("project-members.also-in", { projects: listText(where) })}
        </span>
      )}
    </>
  );
}

/**
 * One person with their share: a box, because only who decides who has which role here opens the
 * page; a person not shown by name has no ID to set a share for, so their share is only said.
 */
export function MemberRow({
  member,
  changes,
}: {
  readonly member: MemberShare;
  readonly changes: ShareChanges;
}) {
  const name = personName(member.person);
  const person = namedPerson(member.person);
  const problem =
    person !== undefined && changes.problem?.person === person.id
      ? changes.problem.message
      : undefined;
  return (
    <li {...partOf(part.item, { variants: ["row"] })}>
      <span {...partOf(part.itemMain)}>
        <span {...partOf(part.itemTitle)}>{name}</span>
        <OverMark member={member} />
      </span>
      {person === undefined ? (
        <span>{percentText(member.percent)}</span>
      ) : (
        <span {...partOf(part.row)}>
          <NumberBox
            name={t("project-members.share-of", { name })}
            value={member.percent}
            problem={problem}
            onCommit={(percent) => {
              if (percent !== null) {
                changes.set(member, percent);
              }
            }}
          />
          <span>{t("common.percent-sign")}</span>
        </span>
      )}
    </li>
  );
}

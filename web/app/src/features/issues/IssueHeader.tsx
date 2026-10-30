import { t } from "@bugaboo/messages";
import {
  ActionMenu,
  Button,
  CopyButton,
  IconButton,
  iconName,
  InPlaceText,
  IssueTypeIcon,
  type MenuAction,
  useLayout,
  partOf,
  part,
} from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { linkTo } from "./issue-actions.ts";
import type { IssueShow } from "./issue-show.ts";
import type { IssueView } from "./slots.ts";

export interface IssueHeaderProps {
  readonly issue: IssueView;
  readonly show: IssueShow;
  /** Whether the person may change the title. */
  readonly canChangeTitle: boolean;
  /** What *More* offers. */
  readonly actions: readonly MenuAction[];
  readonly onTitle: (title: string) => void;
  /** Open in the other way: from the panel to the full page, from the focus view to the panel. */
  readonly onSwitch: () => void;
}

/** The way to the other view: a word in the focus view; in the narrow panel an icon, so the head stays on one row. */
function SwitchButton({ show, onSwitch }: Pick<IssueHeaderProps, "show" | "onSwitch">) {
  if (show === "focus") {
    return <Button onPress={onSwitch}>{t("issues.side-panel-view")}</Button>;
  }
  return (
    <IconButton icon={iconName.expand} label={t("issues.open-full-page")} onPress={onSwitch} />
  );
}

/**
 * The top of an issue (UX §4.4): its type, its ID that copies a link, the title that is changed
 * where it stands, *More* and the way to open it in the other view. The panel has its own *Close*.
 */
export function IssueHeader(props: IssueHeaderProps) {
  const { issue, show, canChangeTitle, actions, onTitle, onSwitch } = props;
  const key = issue.readable_id ?? "";
  const issueType = known(issue.issue_type);
  const Heading = show === "focus" ? "h1" : "h2";
  // A phone always shows the issue on a screen of its own, so there is no other view to open it in.
  const phone = useLayout() === "phone";
  return (
    <>
      <div {...partOf(part.issueHead)}>
        <span {...partOf(part.issueHeadKind)}>
          {issueType === undefined ? null : (
            <>
              <IssueTypeIcon name={issueType.icon} />
              {issueType.name}
            </>
          )}
        </span>
        <CopyButton
          text={linkTo(key)}
          // The panel already shows the ID as its heading; the focus view shows it here.
          label={show === "panel" ? t("issues.copy-link") : key}
          name={t("issues.copy-link-of", { key })}
          copiedLabel={t("issues.link-copied")}
        />
        <div {...partOf(part.issueHeadActions)}>
          {phone ? null : <SwitchButton show={show} onSwitch={onSwitch} />}
          {/* A phone has *More* in the bar of actions under the page: one is enough. */}
          {phone ? null : (
            <ActionMenu
              label={t("issues.more")}
              actions={actions}
              cancelLabel={t("common.cancel")}
            />
          )}
        </div>
      </div>
      <Heading {...partOf(part.issueTitle)}>
        <InPlaceText
          label={t("issue-field.title")}
          buttonLabel={t("issues.change-title-of", { title: issue.title ?? "" })}
          value={issue.title ?? ""}
          required
          readOnly={!canChangeTitle}
          onSave={onTitle}
        />
      </Heading>
    </>
  );
}

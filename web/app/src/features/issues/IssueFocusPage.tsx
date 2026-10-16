import { linesFrom } from "../../shared/paged.ts";
import { FocusView } from "../shell/index.ts";
import { issueShow } from "./issue-show.ts";
import { IssuePage } from "./IssuePage.tsx";
import { lineOf, useIssue, useProjectIssues, type IssueLine } from "./issues.ts";
import { SlimIssueList } from "./SlimIssueList.tsx";

export interface IssueFocusPageProps {
  /** The readable or internal ID in the address. */
  readonly address: string;
  /** Opens another issue in the focus view. */
  readonly onFocus: (key: string) => void;
  /** Back to the list of the project, or to the start page while the project is not known. */
  readonly onBack: (project: string | undefined) => void;
  /** Opens this issue in the side panel next to the list of its project. */
  readonly onPanel: (project: string, key: string) => void;
}

interface InnerProps extends IssueFocusPageProps {
  readonly project: string | undefined;
  readonly lines: readonly IssueLine[];
}

/** The focus view with the slim list around the issue; `j` and `k` (and the buttons) step through it. */
function Focused({ address, project, lines, onFocus, onBack, onPanel }: InnerProps) {
  const at = lines.findIndex((line) => line.key === address || line.id === address);
  const here = lines[at]?.key ?? address;
  const step = (direction: 1 | -1): void => {
    const next = lines[at + direction];
    if (next !== undefined) {
      onFocus(next.key);
    }
  };
  return (
    <FocusView
      list={<SlimIssueList lines={lines} open={here} />}
      onBack={() => {
        onBack(project);
      }}
      onStep={step}
    >
      <IssuePage
        address={address}
        show={issueShow.focus}
        onClose={() => {
          onBack(project);
        }}
        onSwitch={() => {
          if (project !== undefined) {
            onPanel(project, here);
          }
        }}
      />
    </FocusView>
  );
}

/** The slim list needs the project of the issue, so it is read once the issue is known. */
function WithList(props: IssueFocusPageProps & { readonly project: string }) {
  const found = useProjectIssues(props.project);
  return <Focused {...props} lines={linesFrom(found.data?.pages, lineOf)} />;
}

/**
 * An issue in the focus view (UX §3.4): it takes the main area, the sidebar shrinks to icons and a
 * slim list of the project stays on the left. This is the address that is shared
 * (`/issues/WEB-142`).
 */
export function IssueFocusPage(props: IssueFocusPageProps) {
  const project = useIssue(props.address).data?.issue.project?.short_name;
  return project === undefined ? (
    <Focused {...props} project={undefined} lines={[]} />
  ) : (
    <WithList {...props} project={project} />
  );
}

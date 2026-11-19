import { t } from "@bugaboo/messages";
import { Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useApproveAgentRequests } from "./agent-requests.ts";
import { PasswordAgain } from "./PasswordAgain.tsx";

export interface ApproveRequestsProps {
  /** The IDs of the ticked requests that still wait. */
  readonly ticked: readonly string[];
  readonly onApproved: () => void;
}

/** *Approve* the ticked requests with the password once; their keys go to the program that asked. */
export function ApproveRequests({ ticked, onApproved }: ApproveRequestsProps) {
  const [password, setPassword] = useState("");
  const approve = useApproveAgentRequests();
  return (
    <Form
      onSubmit={() => {
        if (ticked.length === 0) {
          return;
        }
        approve.mutate(
          { body: { password, requests: [...ticked] } },
          {
            onSuccess: () => {
              setPassword("");
              onApproved();
            },
          },
        );
      }}
    >
      <PasswordAgain
        help={t("agents.requests-password-help")}
        value={password}
        onChange={setPassword}
        failure={approve.error}
      />
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={approve.isPending}>
          {t("agents.requests-approve", { count: ticked.length })}
        </SubmitButton>
      </div>
    </Form>
  );
}

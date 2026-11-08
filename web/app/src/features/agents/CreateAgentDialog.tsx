import { agentsMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button, buttonLook, TextField } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { agentAddress } from "./agent-address.ts";
import { useCreateAgent } from "./agents.ts";

/**
 * *Create an agent* (UX §4.17): a name and nothing else at first. Its page opens at once, where its
 * keys, its limits and its work are.
 */
export function CreateAgentButton({ count }: { readonly count: number }) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const create = useCreateAgent();
  const navigate = useNavigate();
  const full = count >= agentsMost;
  return (
    <>
      <Button
        look={buttonLook.primary}
        isDisabled={full}
        onPress={() => {
          setName("");
          create.reset();
          setOpen(true);
        }}
      >
        {t("agents.create")}
      </Button>
      <AskDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={t("agents.create")}
        text={t("agents.create-text")}
        confirmLabel={t("agents.create-confirm")}
        cancelLabel={t("common.cancel")}
        confirmDisabled={name.trim().length === 0 || create.isPending}
        confirmHint={t("agents.create-need")}
        onConfirm={() => {
          create.mutate(
            { body: { name: name.trim() } },
            {
              onSuccess: (made: Readonly<{ id?: string | null }>) => {
                void navigate({ href: agentAddress(made.id ?? "") });
              },
            },
          );
        }}
      >
        <TextField label={t("agents.name")} value={name} onChange={setName} />
      </AskDialog>
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {full ? <p>{t("agents.create-full", { most: agentsMost })}</p> : null}
    </>
  );
}

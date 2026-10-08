import type { Schema } from "@bugaboo/api-client";
import { Alert, alertTone, Form, Skeleton } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../../shared/errors.ts";
import { rulesOptions } from "./access-keys.ts";
import { useDraft } from "./draft.ts";
import { Basics, Expiry, Send } from "./FormParts.tsx";
import { MoreOptions } from "./MoreOptions.tsx";
import { useMakeKey } from "./use-make-key.ts";

export interface MakeKeyFormProps {
  /** The key was made: here is the whole key, which is not shown again. */
  readonly onMade: (secret: string) => void;
  readonly onCancel: () => void;
}

interface FieldsProps extends MakeKeyFormProps {
  readonly rules: Schema<"AccessKeyRulesBody">;
}

/** *Make an access key*: a name, what it may do, when it runs out, and the password again. */
function Fields({ rules, onMade, onCancel }: FieldsProps) {
  const draft = useDraft();
  const [needOne, setNeedOne] = useState(false);
  const make = useMakeKey(onMade);
  return (
    <Form
      onSubmit={() => {
        const incomplete = draft.single && draft.picked.length === 0;
        setNeedOne(incomplete);
        if (!incomplete) {
          make.send(draft);
        }
      }}
    >
      <Basics draft={draft} />
      <Expiry draft={draft} rules={rules} />
      <MoreOptions
        draft={draft}
        rules={rules}
        onSwitched={() => {
          setNeedOne(false);
        }}
      />
      <Send
        draft={draft}
        needOne={needOne}
        problem={make.problem}
        isPending={make.isPending}
        onCancel={onCancel}
      />
    </Form>
  );
}

/** The form, once the rules of this organization are known (how long a key may live). */
export function MakeKeyForm(props: MakeKeyFormProps) {
  const rules = useQuery(rulesOptions());
  if (rules.data === undefined) {
    return rules.isError ? (
      <Alert tone={alertTone.danger}>{messageOf(rules.error)}</Alert>
    ) : (
      <Skeleton />
    );
  }
  return <Fields {...props} rules={rules.data} />;
}

// `form` — wraps children + a submit `Action`. Field state lives in
// `page_state` so the resolve loop reflows on input. Mirrors the
// web renderer; on RN we use a `<Column>` instead of `<form>` and
// a `<Button>` `onPress` for submit.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Button, Column } from "@nube/starter-ui-kit-native";
import {
  RenderChildren,
  registerRenderer,
  usePageState,
  useSduiAction,
} from "@nube/starter-ui-sdui-react/headless";

export function RenderForm({ node }: { node: UiComponent }) {
  const submit = node.submit as
    | { handler: string; label?: string; args?: Record<string, unknown> }
    | undefined;
  const [state] = usePageState();
  const action = useSduiAction();
  const onPress = () => {
    if (!submit) return;
    action.mutate({
      handler: submit.handler,
      args: { ...(submit.args ?? {}), page_state: state },
    });
  };
  return (
    <Column gap={12} testID={(node.id as string | undefined) ?? "sdui-form"}>
      <RenderChildren nodes={node.children} />
      {submit ? (
        <Button onPress={onPress} disabled={action.isPending}>
          {submit.label ?? "Submit"}
        </Button>
      ) : null}
    </Column>
  );
}

registerRenderer("form", RenderForm);

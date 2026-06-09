import { beforeEach, describe, expect, it } from "vitest";

import { useUiStore } from "@/store/ui";

// The UI store holds ephemeral client state (edit mode, selection) — not
// server data (F0). Tested by driving the store actions directly.
describe("ui store", () => {
  beforeEach(() => {
    useUiStore.setState({ editMode: false, selectedWidgetId: null });
  });

  it("defaults to view mode with nothing selected", () => {
    const s = useUiStore.getState();
    expect(s.editMode).toBe(false);
    expect(s.selectedWidgetId).toBeNull();
  });

  it("toggles edit mode", () => {
    useUiStore.getState().toggleEditMode();
    expect(useUiStore.getState().editMode).toBe(true);
    useUiStore.getState().toggleEditMode();
    expect(useUiStore.getState().editMode).toBe(false);
  });

  it("leaving edit mode clears the selection", () => {
    useUiStore.getState().setEditMode(true);
    useUiStore.getState().selectWidget("w_1");
    expect(useUiStore.getState().selectedWidgetId).toBe("w_1");
    useUiStore.getState().setEditMode(false);
    expect(useUiStore.getState().selectedWidgetId).toBeNull();
  });
});

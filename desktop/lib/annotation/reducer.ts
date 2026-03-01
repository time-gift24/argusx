import { createLocationFingerprint } from "./location";
import type { AnnotationAction, AnnotationDraft, AnnotationState } from "./state";

export const initialAnnotationState: AnnotationState = {
  items: [],
  activeId: null,
};

function createAnnotationId(): string {
  return `ann-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function getTargetId(state: AnnotationState, actionId?: string): string | null {
  return actionId ?? state.activeId;
}

function updateItem(
  state: AnnotationState,
  targetId: string,
  updater: (item: AnnotationDraft) => AnnotationDraft,
): AnnotationState {
  return {
    ...state,
    items: state.items.map((item) => (item.id === targetId ? updater(item) : item)),
  };
}

export function annotationReducer(
  state: AnnotationState,
  action: AnnotationAction,
): AnnotationState {
  switch (action.type) {
    case "OPEN": {
      const locationFingerprint = createLocationFingerprint(action.location);
      const existing = state.items.find(
        (item) => createLocationFingerprint(item.location) === locationFingerprint,
      );

      if (existing) {
        return {
          ...state,
          activeId: existing.id,
        };
      }

      const nextItems = [...state.items];

      if (state.activeId) {
        const activeIndex = nextItems.findIndex((item) => item.id === state.activeId);
        if (activeIndex >= 0 && nextItems[activeIndex].status !== "submitted") {
          nextItems[activeIndex] = {
            ...nextItems[activeIndex],
            status: "draft",
            updatedAt: Date.now(),
          };
        }
      }

      const draft: AnnotationDraft = {
        id: createAnnotationId(),
        location: action.location,
        ruleCode: null,
        payload: {},
        status: "draft",
        updatedAt: Date.now(),
      };

      nextItems.push(draft);

      return {
        items: nextItems,
        activeId: draft.id,
      };
    }

    case "UPDATE_RULE": {
      const targetId = getTargetId(state, action.id);
      if (!targetId) return state;

      return updateItem(state, targetId, (item) => ({
        ...item,
        ruleCode: action.ruleCode,
        updatedAt: Date.now(),
      }));
    }

    case "UPDATE_PAYLOAD": {
      const targetId = getTargetId(state, action.id);
      if (!targetId) return state;

      return updateItem(state, targetId, (item) => ({
        ...item,
        payload: {
          ...item.payload,
          ...action.payload,
        },
        updatedAt: Date.now(),
      }));
    }

    case "SUBMIT_SUCCESS": {
      const targetId = getTargetId(state, action.id);
      if (!targetId) return state;

      return updateItem(state, targetId, (item) => ({
        ...item,
        status: "submitted",
        updatedAt: Date.now(),
      }));
    }

    case "MARK_ORPHANED": {
      const targetId = getTargetId(state, action.id);
      if (!targetId) return state;

      return updateItem(state, targetId, (item) => ({
        ...item,
        status: "orphaned",
        updatedAt: Date.now(),
      }));
    }

    default:
      return state;
  }
}

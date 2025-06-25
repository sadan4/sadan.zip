import { TAssert } from "@/utils/assert";
import type { Coord } from "@/utils/types";

import { create } from "zustand";
import { devtools } from "zustand/middleware";

export interface CursorContextStore {
    lastMousePos: Coord;
    focusedElement: Element | null;
    clickableElement: Element | null;
    updateFocusedElement(element: Element | null): void;
    updateClickableElement(element: Element | null): void;
}

export const useCursorContextStore = create<CursorContextStore>()(devtools((set) => ({
    lastMousePos: {
        x: 0,
        y: 0,
    },
    focusedElement: null,
    clickableElement: null,
    updateFocusedElement(element) {
        set(() => ({
            focusedElement: element,
        }), undefined, "cursorContext/updateFocusedElement");
    },
    updateClickableElement(element) {
        set(() => ({
            clickableElement: element,
        }), undefined, "cursorContext/updateClickableElement");
    },
}), {
    store: "CursorContextStore",
    name: "CursorContextStore",
    enabled: import.meta.env.DEV,
    actionsDenylist: ["cursorContext/__onMouseMove", "cursorContext/updateClickableElement"],
    trace: false,
}));

document.addEventListener("mousemove", ({ clientX, clientY }) => {
    useCursorContextStore.setState(() => ({
        lastMousePos: {
            x: clientX,
            y: clientY,
        },
    }), undefined, "cursorContext/__onMouseMove");
});
window.addEventListener("focusin", (ev) => {
    TAssert<Element>(ev.target);

    useCursorContextStore
        .getState()
        .updateFocusedElement(ev.target.getAttribute("tabindex") === "-1" ? null : ev.target);
});
window.addEventListener("focusout", () => {
    useCursorContextStore
        .getState()
        .updateFocusedElement(null);
});

import { TAssert } from "@/utils/assert";
import type { Coord } from "@/utils/types";

import { create } from "zustand";

export interface CursorContextStore {
    lastMousePos: Coord;
    focusedElement: Element | null;
    clickableElement: Element | null;
    textElement: Element | null;
    mouseDown: boolean;
    updateFocusedElement(element: Element | null): void;
    updateClickableElement(element: Element | null): void;
    updateTextElement(element: Element | null): void;
}

export const useCursorContextStore = create<CursorContextStore>((set) => ({
    lastMousePos: {
        x: 0,
        y: 0,
    },
    focusedElement: null,
    clickableElement: null,
    textElement: null,
    mouseDown: false,
    updateFocusedElement(element) {
        set(() => ({
            focusedElement: element,
        }));
    },
    updateClickableElement(element) {
        set(() => ({
            clickableElement: element,
        }));
    },
    updateTextElement(element) {
        set(() => ({
            textElement: element,
        }));
    },
}));

document.addEventListener("mousemove", ({ clientX, clientY }) => {
    useCursorContextStore.setState(() => ({
        lastMousePos: {
            x: clientX,
            y: clientY,
        },
    }));
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
window.addEventListener("mousedown", () => {
    useCursorContextStore.setState(() => ({
        mouseDown: true,
    }));
});

window.addEventListener("mouseup", () => {
    useCursorContextStore.setState(() => ({
        mouseDown: false,
    }));
});

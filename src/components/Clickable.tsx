import { useEventHandler } from "@/hooks/eventListener";
import { useUpdatingRef } from "@/hooks/updatingRef";
import type { ElementFromTag } from "@/utils/types";

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentPropsWithRef, createElement, type PropsWithChildren, useEffect } from "react";

export type ClickableProps<T extends "a" | "div" | "span" | "li" = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends "a" | "div" | "span" | "li" = "div">({ tag = "div" as T, ref: _ref, children, ...props }: ClickableProps<T>) {
    const [ref, setRef] = useUpdatingRef<ElementFromTag<T>>();

    useEffect(() => {
        if (typeof _ref === "function") {
            return _ref(ref as any);
        } else if (_ref) {
            _ref.current = ref;
        }
    }, [_ref, ref]);

    useEventHandler("mouseover", function () {
        useCursorContextStore.getState()
            .updateClickableElement(this);
    }, ref);

    useEventHandler("mouseout", () => {
        useCursorContextStore.getState()
            .updateClickableElement(null);
    }, ref);

    return createElement(tag, {
        ref: setRef,
        "data-clickable": "true",
        tabIndex: 0,
        ...props,
    }, children);
}

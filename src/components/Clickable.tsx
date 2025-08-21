import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentPropsWithRef, createElement, type PropsWithChildren, useEffect, useRef } from "react";

export type ClickableProps<T extends "a" | "div" | "span" | "li" = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends "a" | "div" | "span" | "li" = "div">({ tag = "div" as T, onMouseOver, onMouseOut, onMouseUp, children, ...props }: ClickableProps<T>) {
    const shouldNullOnUnmount = useRef(false);

    useEffect(() => {
        return () => {
            if (shouldNullOnUnmount.current) {
                useCursorContextStore.getState()
                    .updateClickableElement(null);
            }
        };
    }, []);

    return createElement(tag, {
        "data-clickable": "true",
        tabIndex: 0,
        onMouseOver(e) {
            shouldNullOnUnmount.current = true;
            useCursorContextStore.getState()
                .updateClickableElement(e.currentTarget as Element);
            onMouseOver?.(e);
        },
        onMouseOut(e) {
            shouldNullOnUnmount.current = false;
            useCursorContextStore.getState()
                .updateClickableElement(null);
            onMouseOut?.(e);
        },
        onMouseUp(e) {
            e.target?.blur();
            onMouseUp?.(e);
        },
        ...props,
    }, children);
}

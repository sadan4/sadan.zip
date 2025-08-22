import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentPropsWithRef, createElement, type PropsWithChildren, useEffect, useRef } from "react";

export type ClickableTags = "a" | "div" | "span" | "li" | "button";

export type ClickableProps<T extends ClickableTags = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends ClickableTags = "div">({ tag = "div" as T, onMouseOver, onMouseOut, onMouseUp, children, ...props }: ClickableProps<T>) {
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
        role: "button",
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

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentPropsWithRef, type PropsWithChildren, useEffect, useRef } from "react";

export type ClickableTags = "a" | "div" | "span" | "li" | "button";

export type ClickableProps<T extends ClickableTags = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends ClickableTags = "div">(_props: ClickableProps<T>) {
    const {
        tag = "div",
        onMouseOver,
        onMouseOut,
        onMouseUp,
        children,
        ...props
    } = _props;

    const shouldNullOnUnmount = useRef(false);
    const Tag = tag as any;

    useEffect(() => {
        return () => {
            if (shouldNullOnUnmount.current) {
                useCursorContextStore.getState()
                    .updateClickableElement(null);
            }
        };
    }, []);

    return (
        <Tag
            data-clickable="true"
            role="button"
            tabIndex={0}
            onMouseOver={(e) => {
                shouldNullOnUnmount.current = true;
                useCursorContextStore.getState()
                    .updateClickableElement(e.currentTarget as Element);
                onMouseOver?.(e);
            }}
            onMouseOut={(e) => {
                shouldNullOnUnmount.current = false;
                useCursorContextStore.getState()
                    .updateClickableElement(null);
                onMouseOut?.(e);
            }}
            onMouseUp={(e) => {
                e.target?.blur();
                onMouseUp?.(e);
            }}
            {...props}
        >
            {children}
        </Tag>
    );
}

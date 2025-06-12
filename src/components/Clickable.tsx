import type { ElementFromTag } from "@/utils/types";

import { type ComponentPropsWithRef, createElement, type PropsWithChildren, useRef } from "react";

export type ClickableProps<T extends "a" | "div" | "span" | "li" = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends "a" | "div" | "span" | "li" = "div">({ tag = "div" as T, ref: _ref, children, ...props }: ClickableProps<T>) {
    const ref = useRef<ElementFromTag<T>>(null);

    return createElement(tag, {
        ref,
        "data-clickable": "true",
        ...props,
    }, children);
}

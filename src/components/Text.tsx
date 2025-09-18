import { base } from "@/styles";
import cn, { textColors, textSize, type TextStyleProps, textWeight } from "@/utils/cn";
import type { ElementFromTag } from "@/utils/types";

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentProps, createElement, type MouseEvent, type PropsWithChildren, useCallback, useEffect, useRef } from "react";

export type TextTags = "div" | "span" | "p";

export interface StandardTextProps extends TextStyleProps {
    noselect?: boolean;
    nowrap?: boolean;
    center?: boolean;
}

export type TextProps<T extends TextTags> = PropsWithChildren<ComponentProps<T>> & StandardTextProps & {
    tag?: T;
};

export function Text<T extends TextTags = "div">(props: TextProps<T>) {
    const {
        className,
        tag = "div",
        size = "sm",
        weight = "normal",
        color = "white",
        children,
        onMouseOver: onMouseOverProp,
        onMouseOut: onMouseOutProp,
        noselect = false,
        nowrap = false,
        center = false,
        ...rest
    } = props;

    type TElement = ElementFromTag<typeof tag>;

    const shouldNullOnUnmount = useRef(false);

    const onMouseOver = useCallback((e: MouseEvent<TElement>) => {
        if (noselect) {
            return;
        }
        onMouseOverProp?.(e as any);
        shouldNullOnUnmount.current = true;
        useCursorContextStore
            .getState()
            .updateTextElement(e.nativeEvent.target as TElement);
    }, [noselect, onMouseOverProp]);

    const onMouseOut = useCallback((e: MouseEvent<TElement>) => {
        if (noselect) {
            return;
        }
        onMouseOutProp?.(e as any);
        shouldNullOnUnmount.current = false;
        useCursorContextStore
            .getState()
            .updateTextElement(null);
    }, [noselect, onMouseOutProp]);

    useEffect(() => {
        return () => {
            if (shouldNullOnUnmount.current) {
                useCursorContextStore.getState()
                    .updateTextElement(null);
            }
        };
    }, []);

    const el = createElement(tag, {
        className: cn(
            "text",
            base.wFit,
            noselect && "select-none",
            nowrap && "whitespace-nowrap",
            textSize[size],
            textWeight[weight],
            textColors[color],
            className,
        ),
        onMouseOver,
        onMouseOut,
        ...rest,
    }, children);

    if (center) {
        return <div className="flex justify-center">{el}</div>;
    }
    return el;
}

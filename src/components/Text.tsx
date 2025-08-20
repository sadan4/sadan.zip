import cn from "@/utils/cn";
import type { ElementFromTag } from "@/utils/types";

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentProps, createElement, type MouseEvent, type PropsWithChildren, useCallback, useEffect, useRef } from "react";

export type TextTags = "div" | "span" | "p";

const textSize = {
    xs: "text-xs",
    sm: "text-sm",
    md: "text-base",
    lg: "text-lg",
    xl: "text-xl",
    "2xl": "text-2xl",
    "3xl": "text-3xl",
    "4xl": "text-4xl",
    "5xl": "text-5xl",
    "6xl": "text-6xl",
    "7xl": "text-7xl",
    "8xl": "text-8xl",
    "9xl": "text-9xl",
};

const textWeight = {
    thin: "font-thin",
    extraLight: "font-extralight",
    light: "font-light",
    normal: "font-normal",
    medium: "font-medium",
    semiBold: "font-semibold",
    bold: "font-bold",
    extraBold: "font-extrabold",
    black: "font-black",
};

const textColor = {
    black: "text-bg-100",
    "black-200": "text-bg-200",
    "black-300": "text-bg-300",
    white: "text-bg-fg",
    "white-600": "text-bg-fg-600",
    "white-700": "text-bg-fg-700",
    "white-800": "text-bg-fg-800",
    "white-900": "text-bg-fg-900",
    primary: "text-primary",
    secondary: "text-secondary",
    accent: "text-accent",
    neutral: "text-neutral",
    info: "text-info",
    "info-600": "text-info-600",
    "info-700": "text-info-700",
    success: "text-success",
    warning: "text-warning",
    error: "text-error",
};

export type TextProps<T extends TextTags> = PropsWithChildren<ComponentProps<T>> & {
    tag?: T;
    size?: keyof typeof textSize;
    weight?: keyof typeof textWeight;
    color?: keyof typeof textColor;
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
        ...rest
    } = props;

    type TElement = ElementFromTag<typeof tag>;

    const shouldNullOnUnmount = useRef(false);

    const onMouseOver = useCallback((e: MouseEvent<TElement>) => {
        onMouseOverProp?.(e as any);
        shouldNullOnUnmount.current = true;
        useCursorContextStore
            .getState()
            .updateTextElement(e.nativeEvent.target as TElement);
    }, [onMouseOverProp]);

    const onMouseOut = useCallback((e: MouseEvent<TElement>) => {
        onMouseOutProp?.(e as any);
        shouldNullOnUnmount.current = false;
        useCursorContextStore
            .getState()
            .updateTextElement(null);
    }, [onMouseOutProp]);

    useEffect(() => {
        return () => {
            if (shouldNullOnUnmount.current) {
                useCursorContextStore.getState()
                    .updateTextElement(null);
            }
        };
    }, []);

    return createElement(tag, {
        className: cn("text", textSize[size], textWeight[weight], textColor[color], className),
        onMouseOver,
        onMouseOut,
        ...rest,
    }, children);
}

import { base } from "@/styles";
import cn, { type SizeProp, textSize, textWeight, type WeightProp } from "@/utils/cn";
import type { ElementFromTag } from "@/utils/types";

import styles from "./styles.module.css";
import { useCursorContextStore } from "../Cursor/cursorContextStore";

import { type ComponentProps, type MouseEvent, type PropsWithChildren, useCallback, useEffect, useRef } from "react";

const textColors = {
    black: styles.black,
    "black-200": styles.black200,
    "black-300": styles.black300,
    white: styles.white,
    "white-600": styles.white600,
    "white-700": styles.white700,
    "white-800": styles.white800,
    primary: styles.primary,
    secondary: styles.secondary,
    accent: styles.accent,
    neutral: styles.neutral,
    "neutral-content": styles.neutralContent,
    info: styles.info,
    "info-600": styles.info600,
    "info-700": styles.info700,
    success: styles.success,
    warning: styles.warning,
    error: styles.error,
} as const;

export type TextTags = "div" | "span" | "p";

export interface StandardTextProps extends SizeProp, WeightProp {
    /**
     * disallow text selection
     */
    noselect?: boolean;
    /**
     * Prevent text from wrapping
     */
    nowrap?: boolean;
    /**
     * Center the text
     */
    center?: boolean;
    /**
     * The color of the text
     */
    color?: keyof typeof textColors;
}

export type TextProps<T extends TextTags> = PropsWithChildren<ComponentProps<T>> & StandardTextProps & {
    tag?: T;
};


/**
 * Standard text component
 */
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

    const Tag = tag as any;

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

    const el = (
        <Tag
            className={
                cn(
                    "text",
                    base.wFit,
                    noselect && "select-none",
                    nowrap && "whitespace-nowrap",
                    textSize[size],
                    textWeight[weight],
                    textColors[color],
                    className,
                )
            }
            onMouseOver={onMouseOver}
            onMouseOut={onMouseOut}
            {...rest}
        >{children}
        </Tag>
    );

    if (center) {
        return <div className="flex justify-center">{el}</div>;
    }
    return el;
}

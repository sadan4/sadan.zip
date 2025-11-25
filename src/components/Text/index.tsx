import cn, { type SizeProp, textSize, textWeight, type WeightProp } from "@/utils/cn";

import styles from "./styles.module.scss";

import { type ComponentProps, type PropsWithChildren } from "react";

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
    "info-400": styles.info400,
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

export type TextProps<T extends TextTags = "div"> = PropsWithChildren<ComponentProps<T>> & StandardTextProps & {
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
        noselect = false,
        nowrap = false,
        center = false,
        ...rest
    } = props;

    const Tag = tag as any;

    const el = (
        <Tag
            className={
                cn(
                    "text",
                    noselect && "select-none",
                    nowrap && "whitespace-nowrap",
                    textSize[size],
                    textWeight[weight],
                    textColors[color],
                    className,
                )
            }
            {...rest}
        >{children}
        </Tag>
    );

    if (center) {
        return <div className="flex justify-center">{el}</div>;
    }
    return el;
}

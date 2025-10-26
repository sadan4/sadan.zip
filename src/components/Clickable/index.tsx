import cn from "@/utils/cn";

import styles from "./styles.module.scss";

import { type ComponentPropsWithRef, type PropsWithChildren } from "react";

export type ClickableTags = "a" | "div" | "span" | "li" | "button";

export type ClickableProps<T extends ClickableTags = "div"> = PropsWithChildren<ComponentPropsWithRef<T>> & {
    tag?: T | undefined;
};

export function Clickable<T extends ClickableTags = "div">(_props: ClickableProps<T>) {
    const {
        tag = "div",
        onMouseUp,
        children,
        className,
        ...props
    } = _props;

    const Tag = tag as any;

    return (
        <Tag
            role={tag !== "a" ? "button" : undefined}
            tabIndex={0}
            // TODO: type this
            onMouseUp={(e: any) => {
                e.target?.blur();
                onMouseUp?.(e);
            }}
            className={cn(styles.clickable, className)}
            {...props}
        >
            {children}
        </Tag>
    );
}

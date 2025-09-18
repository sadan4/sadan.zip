import { buttonColors, buttonTextColors, cn, type textSize } from "@/utils/cn";

import { Clickable } from "./Clickable";
import styles from "./styles.module.css";
import { Text } from "./Text";

import type { ComponentProps } from "react";

export interface ButtonProps extends ComponentProps<typeof Clickable<"button">> {
    color?: keyof typeof buttonColors;
    size?: keyof typeof textSize;
    wrap?: boolean;
}

export function Button({ children, className, color = "primary", size = "md", wrap = false, ...props }: ButtonProps) {
    return (
        <Clickable
            className={cn(styles.button, buttonColors[color], className)}
            {...props}
            tag="button"
        >
            <Text
                size={size}
                color={buttonTextColors[color]}
                nowrap={!wrap}
                noselect
            >
                {children}
            </Text>
        </Clickable>
    );
}

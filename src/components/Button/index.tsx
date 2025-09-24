import { cn, type textSize } from "@/utils/cn";

import { colors, colorTypes } from "./colors";
import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import { Text } from "../Text";

import type { ComponentProps } from "react";


export interface ButtonProps extends ComponentProps<typeof Clickable<"button">> {
    /**
     * The color of the button
     */
    color?: keyof typeof colors;
    /**
     * The size of the button text
     */
    size?: keyof typeof textSize;
    /**
     * Whether the button text should wrap
     *
     * @default false
     */
    wrap?: boolean;
    /**
     * The style of the button
     */
    colorType?: keyof typeof colorTypes;
    /**
     * If the button is disabled
     */
    disabled?: boolean;
}

/**
 * A simple button
 */
export function Button({ children, className, color = "primary", size = "md", wrap = false, colorType = "filled", disabled = false, ...props }: ButtonProps) {
    return (
        <Clickable
            className={cn(styles.button, colors[color], colorTypes[colorType], className)}
            {...props}
            disabled={disabled}
            tag="button"
        >
            <Text
                size={size}
                nowrap={!wrap}
                className={styles.buttonText}
                noselect
            >
                {children}
            </Text>
        </Clickable>
    );
}

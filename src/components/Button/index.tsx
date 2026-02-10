import { useDebouncedFn } from "@/hooks/debouncedFn";
import { cn, type textSize } from "@/utils/cn";
import type { Thenable } from "@/utils/types";

import { colors, colorTypes } from "./colors";
import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import { Text } from "../Text";
import { Tooltip } from "../Tooltip";
import type { TooltipPosition } from "../Tooltip/constants";

import { CheckIcon, XIcon } from "lucide-react";
import { type ComponentProps, type MouseEvent, useId, useState } from "react";

export interface BaseButtonProps extends ComponentProps<typeof Clickable<"button">> {
    /**
     * The color of the button
     */
    color?: keyof typeof colors;
    /**
     * The style of the button
     */
    colorType?: keyof typeof colorTypes;
    /**
     * If the button is disabled
     */
    disabled?: boolean;
    /**
     * The accessible label for the button
     */
    label?: string;
}

export interface ButtonProps extends BaseButtonProps {
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
}


/**
 * A simple button
 */
export function Button({ children, className, color = "primary", size = "md", wrap = false, colorType = "filled", disabled = false, label, ...props }: ButtonProps) {
    const id = useId();

    return (
        <Clickable
            className={cn(styles.button, colors[color], colorTypes[colorType], className)}
            {...props}
            disabled={disabled}
            tag="button"
            {...label != null ? { "aria-label": label } : { "aria-labelledby": id }}
        >
            <Text
                id={id}
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

export interface IconButtonProps extends BaseButtonProps {
    /**
     * return `true` to show a success animation
     */
    onClick: ((event: MouseEvent<HTMLButtonElement>) => Thenable<boolean | null>) | undefined;
    /**
     * label for button
     */
    label: string;
    /**
     * duration to hold success animation in ms
     * 
     * @default 750
     */
    holdAnimDuration?: number;
    /**
     * whether to dispatch the onClick during animation frame
     * 
     * @default true
     */
    dispatchDuringAnim?: boolean;
    /**
     * color of the check icon
     * 
     * @default "success"
     */
    checkColor?: BaseButtonProps["color"];
    tooltipPosition?: TooltipPosition;
}


export function IconButton({
    children,
    className,
    color = "primary",
    checkColor = "success",
    colorType = "filled",
    disabled = false,
    label,
    onClick,
    tooltipPosition,
    holdAnimDuration = 750,
    dispatchDuringAnim = true,
    ...props
}: IconButtonProps) {
    const enum ButtonState {
        GOOD,
        BAD,
        NORMAL,
    }

    const [showCheck, setShowCheck] = useState(ButtonState.NORMAL);

    const hideCheck = useDebouncedFn(() => {
        setShowCheck(ButtonState.NORMAL);
    }, holdAnimDuration, true);

    return (
        <Tooltip
            text={label}
            position={tooltipPosition}
        >
            <Clickable
                className={cn(
                    className,
                    styles.button,
                    styles.iconButton,
                    colors[color],
                    colorTypes[colorType],
                    showCheck === ButtonState.GOOD && styles.showCheck,
                    showCheck === ButtonState.BAD && styles.showError,
                )}
                {...props}
                disabled={disabled}
                onClick={(e) => {
                    if (showCheck && !dispatchDuringAnim) {
                        return;
                    }
                    Promise.resolve(onClick?.(e)).then((result) => {
                        if (result === true) {
                            setShowCheck(ButtonState.GOOD);
                            hideCheck();
                        } else if (result === false) {
                            setShowCheck(ButtonState.BAD);
                            hideCheck();
                        }
                    });
                }}
                tag="button"
            >
                <div className={styles.icon}>
                    {children}
                </div>
                <div className={cn(styles.statusIcon, styles.checkIcon, colors[checkColor])}>
                    <CheckIcon />
                </div>
                <div className={cn(styles.statusIcon, styles.errorIcon, colors.error)}>
                    <XIcon />
                </div>
            </Clickable>
        </Tooltip>
    );
}

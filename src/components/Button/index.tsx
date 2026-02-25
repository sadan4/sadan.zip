import { useDebouncedFn } from "@/hooks/debouncedFn";
import { cn, type textSize } from "@/utils/cn";
import type { Thenable } from "@/utils/types";
import { TAssert } from "@vencord-companion/webpack-ast-parser/util";

import { colors, colorTypes } from "./colors";
import styles from "./styles.module.scss";
import { Clickable, type ClickableTags } from "../Clickable";
import { Text } from "../Text";
import { Tooltip } from "../Tooltip";
import type { TooltipPosition } from "../Tooltip/constants";

import { CheckIcon, Loader2Icon, XIcon } from "lucide-react";
import { type ComponentProps, type MouseEvent, useId, useState } from "react";

export type BaseButtonProps<Tag extends ClickableTags> = ComponentProps<typeof Clickable<Tag>> & {
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
};

export interface ButtonProps extends BaseButtonProps<"button"> {
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

export type IconButtonProps<Tag extends "a" | "button"> = BaseButtonProps<Tag> & {
    /**
     * return `true` to show a success animation
     */
    onClick: ((event: MouseEvent<Tag extends "a" ? HTMLAnchorElement : HTMLButtonElement>) => Thenable<boolean | null>) | undefined;
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
     * @default false
     */
    dispatchDuringAnim?: boolean;
    /**
     * color of the check icon
     * 
     * @default "success"
     */
    checkColor?: BaseButtonProps<Tag>["color"];
    /**
     * show a loading animation while the promise is pending
     * 
     * @default false
     */
    loadingAnimation?: boolean;
    tooltipPosition?: TooltipPosition;
    tooltipClassName?: string;
    tag?: Tag;
};


export function IconButton<T extends "a" | "button" = "button">({
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
    loadingAnimation = false,
    dispatchDuringAnim = false,
    tag = "button" as T,
    tooltipClassName,
    ...props
}: IconButtonProps<T>) {
    const enum ButtonState {
        GOOD,
        BAD,
        LOADING,
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
            tooltipClassName={tooltipClassName}
        >
            <Clickable<"button">
                className={cn(
                    className,
                    styles.button,
                    styles.iconButton,
                    colors[color],
                    colorTypes[colorType],
                    showCheck === ButtonState.GOOD && styles.showCheck,
                    showCheck === ButtonState.BAD && styles.showError,
                    showCheck === ButtonState.LOADING && styles.showLoading,
                    showCheck !== ButtonState.NORMAL && !dispatchDuringAnim && "cursor-not-allowed",
                )}
                {...props as any}
                tag={tag as "button"}
                disabled={disabled}
                onClick={(e: unknown) => {
                    if (showCheck !== ButtonState.NORMAL && !dispatchDuringAnim) {
                        return;
                    }
                    TAssert<MouseEvent<T extends "a" ? HTMLAnchorElement : HTMLButtonElement>>(e);

                    const res = onClick?.(e);

                    if (res === null) {
                        // noop
                    } else if (typeof res === "boolean") {
                        setShowCheck(res ? ButtonState.GOOD : ButtonState.BAD);
                        hideCheck();
                    } else {
                        // promise
                        setShowCheck(loadingAnimation ? ButtonState.LOADING : ButtonState.NORMAL);
                        Promise.resolve(res).then((result) => {
                            TAssert<boolean | null>(result);
                            if (result === true) {
                                setShowCheck(ButtonState.GOOD);
                                hideCheck();
                            } else if (result === false) {
                                setShowCheck(ButtonState.BAD);
                                hideCheck();
                            } else {
                                setShowCheck(ButtonState.NORMAL);
                            }
                        });
                    }
                }}
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
                <div className={cn(styles.statusIcon, styles.loadingIcon)}>
                    <Loader2Icon />
                </div>
            </Clickable>
        </Tooltip>
    );
}

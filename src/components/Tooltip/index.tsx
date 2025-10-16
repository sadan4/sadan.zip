import { useControlledState } from "@/hooks/controlledState";
import { useForceUpdater } from "@/hooks/forceUpdater";
import cn from "@/utils/cn";
import { updateRef } from "@/utils/ref";
import useResizeObserver from "@react-hook/resize-observer";
import { animated, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import styles from "./styles.module.scss";
import { Box } from "../layout/Box";

import { type ComponentProps, type ReactNode, useLayoutEffect, useRef } from "react";
import { measureRect } from "@/utils/dom";

export interface TooltipProps extends ComponentProps<"div"> {
    /**
     * The content of the tooltip
     */
    text: ReactNode;
    /**
     * The position of the tooltip, leave blank for default (TOP)
     */
    position?: TooltipPosition;
    show?: boolean;
    onShow?(): void;
    onHide?(): void;
    className?: string;
    triggerProps?: ComponentProps<"div">;
    /**
     * Don't use the default wrapper ({@link Box})
     */
    noWrapper?: boolean;
    /**
     * Delay in ms before showing the tooltip on hover
     */
    hoverShowDelay?: number;
}

function useTooltipAnim(shouldShow: boolean) {
    const scaleBy = 0.1;

    return useTransition(shouldShow, {
        from: {
            opacity: 0,
            scale: 0.95,
            "--percent-in": -scaleBy,
        },
        enter: {
            opacity: 1,
            scale: 1,
            "--percent-in": 0,
        },
        leave: {
            opacity: 0,
            scale: 0.95,
            "--percent-in": scaleBy,
        },
        config: {
            tension: 2400,
            friction: 52,
        },
    });
}

const posMap: Record<TooltipPosition, string> = {
    [TooltipPosition.TOP]: styles.top,
    [TooltipPosition.BOTTOM]: styles.bottom,
    [TooltipPosition.LEFT]: styles.left,
    [TooltipPosition.RIGHT]: styles.right,
};

export function Tooltip({
    text,
    show: _show,
    onShow,
    onHide,
    className,
    triggerProps,
    position = TooltipPosition.TOP,
    children,
    noWrapper = false,
    hoverShowDelay,
    ref,
    ...props
}: TooltipProps) {
    const [shouldShow, setShouldShow] = useControlledState({
        initialValue: false,
        managedValue: _show,
        handleChange: (s) => (s ? onShow : onHide)?.(),
        debugName: "Tooltip",
    });

    const timeoutRef = useRef<NodeJS.Timeout>(undefined);
    const triggerRef = useRef<HTMLDivElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const [dep, updateSizeVar] = useForceUpdater();

    useResizeObserver(triggerRef, updateSizeVar);

    useLayoutEffect(() => {
        if (triggerRef.current && containerRef.current) {
            const { width, height } = measureRect(triggerRef.current);

            containerRef.current.style.setProperty("--trigger-width", `${width}px`);
            containerRef.current.style.setProperty("--trigger-height", `${height}px`);
        }
    }, [dep]);

    const tooltipTransition = useTooltipAnim(shouldShow);

    const show = () => {
        if (hoverShowDelay == null) {
            setShouldShow(true);
        }
        clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => {
            setShouldShow(true);
        }, hoverShowDelay);
    };

    const hide = () => {
        clearTimeout(timeoutRef.current);
        setShouldShow(false);
    };

    return (
        <div
            {...props}
            className={cn(styles.tooltip, className)}
            onMouseEnter={show}
            onMouseLeave={hide}
            ref={(value) => {
                updateRef(containerRef, value);
                updateRef(ref, value);
            }}
        >
            {
                tooltipTransition(({ ...styleProps }, show) => {
                    return show && (
                        <animated.div
                            className={cn(styles.container, posMap[position])}
                            style={{
                                ...styleProps,
                            }}
                        >
                            {noWrapper
                                ? text
                                : (
                                    <Box className={styles.box}>
                                        <div className={styles.wrapper}>
                                            {text}
                                        </div>
                                    </Box>
                                )}
                        </animated.div>
                    );
                })
            }
            <div
                {...triggerProps}
                ref={(value) => {
                    updateRef(triggerProps?.ref, value);
                    updateRef(triggerRef, value);
                }}
                className={cn(styles.trigger, triggerProps?.className)}
            >
                {children}
            </div>
        </div>
    );
}

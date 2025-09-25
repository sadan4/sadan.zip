import { useControlledState } from "@/hooks/controlledState";
import { useForceUpdater } from "@/hooks/forceUpdater";
import cn from "@/utils/cn";
import useResizeObserver from "@react-hook/resize-observer";
import { animated, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import styles from "./styles.module.scss";
import { Box } from "../layout/Box";

import { type PropsWithChildren, type ReactNode, useLayoutEffect, useRef } from "react";

export interface TooltipProps extends PropsWithChildren {
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
    position = TooltipPosition.TOP,
    children,
    noWrapper = false,
    hoverShowDelay,
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
            const { width, height } = triggerRef.current.getBoundingClientRect();

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
            className={cn(styles.tooltip, className)}
            onMouseEnter={show}
            onMouseLeave={hide}
            ref={containerRef}
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
                ref={triggerRef}
                className={cn(styles.trigger)}
            >
                {children}
            </div>
        </div>
    );
}

import { useComposedRefs } from "@/hooks/composedRefs";
import { useControlledState } from "@/hooks/controlledState";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useReiszeObserverFromRef } from "@/hooks/resizeObserver";
import cn from "@/utils/cn";
import { measureRect } from "@/utils/dom";
import { unreachable } from "@/utils/error";
import { updateRef } from "@/utils/ref";
import { animated, to, useSpringValue, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import styles from "./styles.module.scss";
import { LayerPortal } from "../Layer";
import { Box } from "../layout/Box";

import { type ComponentProps, type ReactNode, useLayoutEffect, useRef } from "react";

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
     *
     * @default 250
     */
    hoverShowDelay?: number;
    /**
     * @default 250
     */
    lingerDelay?: number;
    /**
     * Don't show the arrow point to the trigger element
     *
     * @default false
     */
    noarrow?: boolean;
    tooltipClassName?: string;
}

declare module "react" {
    interface CSSProperties {
        "--pad"?: string;
    }
}

function useTooltipAnim(shouldShow: boolean) {
    const scaleBy = 0.1;

    return useTransition(shouldShow, {
        from: {
            opacity: 0,
            scale: 0.95,
            percentIn: -scaleBy,
        },
        enter: {
            opacity: 1,
            scale: 1,
            percentIn: 0,
        },
        leave: {
            opacity: 0,
            scale: 0.95,
            percentIn: scaleBy,
        },
        config: {
            tension: 2400,
            friction: 52,
        },
    });
}

function TooltipArrow() {
    return (
        <svg
            height="24"
            width="24"
            viewBox="0 0 24 24"
            className={styles.arrow}
        >
            <path
                className={styles.border}
                d="m8.96,8.98l-8.96,-8.98l23.99,0l-8.92,8.98a3.53,2.05 180 0 1 -6.11,0z"
            />
            <path
                d="m8.96,8.98l-8.96,-8.98l23.99,0l-8.92,8.98a3.53,2.05 180 0 1 -6.11,0z"
            />
        </svg>
    );
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
    hoverShowDelay = 250,
    lingerDelay = 250,
    noarrow = noWrapper,
    tooltipClassName,
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
    const triggerHeight = useSpringValue(0);
    const triggerWidth = useSpringValue(0);
    const [dep, updateSizeVar] = useForceUpdater();

    useReiszeObserverFromRef(triggerRef, updateSizeVar);

    useLayoutEffect(() => {
        if (triggerRef.current && containerRef.current) {
            const { width, height } = measureRect(triggerRef.current);

            if (!triggerHeight.hasAnimated) {
                triggerHeight.set(height);
                triggerWidth.set(width);
            } else {
                triggerHeight.start(height);
                triggerWidth.start(width);
            }
        }
    }, [dep, triggerHeight, triggerWidth]);

    const tooltipTransition = useTooltipAnim(shouldShow);

    const show = () => {
        clearTimeout(timeoutRef.current);
        if (!hoverShowDelay) {
            setShouldShow(true);
        } else {
            timeoutRef.current = setTimeout(() => {
                setShouldShow(true);
            }, hoverShowDelay);
        }
    };

    const hide = () => {
        clearTimeout(timeoutRef.current);
        if (!lingerDelay) {
            setShouldShow(false);
        } else {
            timeoutRef.current = setTimeout(() => {
                setShouldShow(false);
            }, lingerDelay);
        }
    };

    return (
        <div
            {...props}
            className={cn(styles.tooltip, className)}
            onMouseEnter={show}
            onMouseLeave={hide}
            ref={useComposedRefs(ref, containerRef)}
        >
            <LayerPortal>
                {
                    // FIXME: vvv
                    // eslint-disable-next-line react-hooks/refs
                    tooltipTransition(({ percentIn, ...styleProps }, show) => {
                        const triggerRect = triggerRef.current && measureRect(triggerRef.current);

                        return show && triggerRect && (
                            <animated.div
                                className={cn(styles.container, posMap[position], tooltipClassName)}
                                style={{
                                    ...styleProps,
                                    ...(() => {
                                        const { top, left, width, height } = triggerRect;

                                        switch (position) {
                                            case TooltipPosition.TOP: {
                                                const paddingBottom = to(
                                                    [percentIn, triggerHeight],
                                                    (percentIn, triggerHeight) => `calc(1rem + ${percentIn * triggerHeight}px)`,
                                                );

                                                return {
                                                    left: left + (width / 2),
                                                    top,
                                                    paddingBottom,
                                                    "--pad": paddingBottom,
                                                };
                                            }
                                            case TooltipPosition.BOTTOM: {
                                                const paddingTop = to(
                                                    [percentIn, triggerHeight],
                                                    (percentIn, triggerHeight) => `calc(1rem + ${percentIn * triggerHeight}px)`,
                                                );

                                                return {
                                                    left: left + (width / 2),
                                                    top: height + top,
                                                    paddingTop,
                                                    "--pad": paddingTop,
                                                };
                                            }
                                            case TooltipPosition.LEFT: {
                                                const paddingRight = to(
                                                    [percentIn, triggerWidth],
                                                    (percentIn, triggerWidth) => `calc(1rem + ${percentIn * triggerWidth}px)`,
                                                );

                                                return {
                                                    top: top + (height / 2),
                                                    left,
                                                    paddingRight,
                                                    "--pad": paddingRight,
                                                };
                                            }
                                            case TooltipPosition.RIGHT: {
                                                const paddingLeft = to(
                                                    [percentIn, triggerWidth],
                                                    (percentIn, triggerWidth) => `calc(1rem + ${percentIn * triggerWidth}px)`,
                                                );

                                                return {
                                                    top: top + (height / 2),
                                                    left: left + width,
                                                    paddingLeft,
                                                    "--pad": paddingLeft,
                                                };
                                            }

                                            default: {
                                                unreachable();
                                            }
                                        }
                                    })(),
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
                                {noarrow || <TooltipArrow />}
                            </animated.div>
                        );
                    })
                }
            </LayerPortal>
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

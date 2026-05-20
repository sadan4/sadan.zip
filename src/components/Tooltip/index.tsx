import { useComposedRefs } from "@/hooks/composedRefs";
import { useControlledState } from "@/hooks/controlledState";
import cn from "@/utils/cn";
import { EMPTY_OBJECT } from "@/utils/constants";
import { measureRect } from "@/utils/dom/rect";
import type { TOmit } from "@/utils/types";
import { arrow, autoPlacement, flip, FloatingArrow, offset, type Placement, shift, useFloating } from "@floating-ui/react";
import { animated, type AnimatedProps, type SpringValue, to, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import * as styles from "./styles.module.scss";
import { LayerPortal } from "../Layer";
import { Box } from "../layout/Box";

import { type ComponentProps, type CSSProperties, type ReactNode, useRef } from "react";

interface MakeTooltipPositionStylesOptions {
    position: Exclude<TooltipPosition, TooltipPosition.AUTO>;
    percentIn: SpringValue<number>;
    triggerHeight: number;
    triggerWidth: number;
}

function makeTooltipPositionStyles({
    position,
    percentIn,
    triggerHeight,
    triggerWidth,
}: MakeTooltipPositionStylesOptions): AnimatedProps<CSSProperties> {
    switch (position) {
        case TooltipPosition.TOP: {
            const paddingBottom = to(
                [percentIn, triggerHeight],
                (percentIn, triggerHeight) => `calc(${percentIn * triggerHeight}px)`,
            );

            return {
                paddingBottom,
            };
        }
        case TooltipPosition.BOTTOM: {
            const paddingTop = to(
                [percentIn, triggerHeight],
                (percentIn, triggerHeight) => `calc(${percentIn * triggerHeight}px)`,
            );

            return {
                paddingTop,
            };
        }
        case TooltipPosition.LEFT: {
            const paddingRight = to(
                [percentIn, triggerWidth],
                (percentIn, triggerWidth) => `calc(${percentIn * triggerWidth}px)`,
            );

            return {
                paddingRight,
            };
        }
        case TooltipPosition.RIGHT: {
            const paddingLeft = to(
                [percentIn, triggerWidth],
                (percentIn, triggerWidth) => `calc(${percentIn * triggerWidth}px)`,
            );

            return {
                paddingLeft,
            };
        }
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
    triggerProps?: TOmit<ComponentProps<"div">, "children">;
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
    noArrow?: boolean;
    tooltipClassName?: string;
    /**
     * offset from trigger in px
     * @default 8
     */
    offset?: number;
}

const floatingPosotionMap = /* @__PURE__ */ Object.freeze({
    [TooltipPosition.TOP]: "top",
    [TooltipPosition.BOTTOM]: "bottom",
    [TooltipPosition.LEFT]: "left",
    [TooltipPosition.RIGHT]: "right",
    [TooltipPosition.AUTO]: undefined,
} satisfies Record<TooltipPosition, Placement | undefined>);

export function Tooltip({
    text,
    show: _show,
    onShow,
    onHide,
    className,
    triggerProps: { ref: _triggerRef, className: triggerClassName, ...triggerProps } = EMPTY_OBJECT,
    position = TooltipPosition.TOP,
    children,
    noWrapper = false,
    hoverShowDelay = 250,
    lingerDelay = 250,
    noArrow = noWrapper,
    tooltipClassName,
    offset: offsetLen = 8,
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
    const arrowRef = useRef<SVGSVGElement>(null);


    const {
        refs: {
            setFloating: setTooltipRef,
            setReference: setTriggerEl,
            domReference: triggerEl,
        },
        context,
        floatingStyles,
    } = useFloating<HTMLDivElement>({
        placement: floatingPosotionMap[position],
        middleware: [
            offset(offsetLen),
            ...floatingPosotionMap[position]
                ? [
                    shift({
                        padding: offsetLen,
                    }),
                    flip(),
                ]
                : [autoPlacement()],
            // eslint-disable-next-line react-hooks/refs
            arrow({
                element: arrowRef,
            }),
        ],
    });

    const tooltipTransition = useTooltipAnim(shouldShow);

    function show() {
        clearTimeout(timeoutRef.current);
        if (!hoverShowDelay) {
            setShouldShow(true);
        } else {
            timeoutRef.current = setTimeout(() => {
                setShouldShow(true);
            }, hoverShowDelay);
        }
    }

    function hide() {
        clearTimeout(timeoutRef.current);
        if (!lingerDelay) {
            setShouldShow(false);
        } else {
            timeoutRef.current = setTimeout(() => {
                setShouldShow(false);
            }, lingerDelay);
        }
    }

    return (
        <div
            {...props}
            className={cn(styles.tooltip, className)}
            onMouseEnter={show}
            onMouseLeave={hide}
            ref={ref}
        >
            <LayerPortal>
                {tooltipTransition(({ percentIn, ...styleProps }, show) => {
                    const el = triggerEl.current;
                    const triggerRect = el && measureRect(el);

                    const animStyles = makeTooltipPositionStyles({
                        percentIn,
                        position: position as any,
                        triggerHeight: triggerRect?.height ?? 0,
                        triggerWidth: triggerRect?.width ?? 0,
                    });

                    return show && (
                        <div
                            className={tooltipClassName}
                            style={floatingStyles}
                            ref={setTooltipRef}
                        >
                            <animated.div
                                style={{
                                    ...styleProps,
                                    ...animStyles,
                                }}
                                className={styles.container}
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
                                {noArrow || (
                                    <FloatingArrow
                                        ref={arrowRef}
                                        context={context}
                                        strokeWidth={2}
                                        tipRadius={2}
                                        className={styles.arrow}
                                    />
                                )}
                            </animated.div>
                        </div>
                    );
                })}
            </LayerPortal>
            <div
                {...triggerProps}
                ref={useComposedRefs(setTriggerEl, _triggerRef)}
                className={cn(styles.trigger, triggerClassName)}
            >
                {children}
            </div>
        </div>
    );
}

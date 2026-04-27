import { useComposedRefs } from "@/hooks/composedRefs";
import { useControlledState } from "@/hooks/controlledState";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useIntersection } from "@/hooks/intersection";
import { useRecent } from "@/hooks/recent";
import { useFragmentRect, useRect } from "@/hooks/rect";
import { useResizeObserver } from "@/hooks/resizeObserver";
import cn from "@/utils/cn";
import { EMPTY_OBJECT } from "@/utils/constants";
import {
    compareRectOffsets,
    measureRect,
    mergeRectOffsets,
    NO_OFFSET,
    rectFullyContainedBy,
    rectHeightCanBeContainedBy,
    type RectOffset,
    rectWidthCanBeContainedBy,
    removeMarginFromRect,
} from "@/utils/dom/rect";
import { measureFragmentRect } from "@/utils/react";
import type { TOmit } from "@/utils/types";
import { animated, type AnimatedProps, type SpringValue, to, useSpringValue, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import styles from "./styles.module.scss";
import { LayerPortal } from "../Layer";
import { LayerContext } from "../Layer/context";
import { Box } from "../layout/Box";

import { type ComponentProps, type CSSProperties, Fragment, type FragmentInstance, type ReactNode, type RefObject, use, useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";

declare module "react" {
    interface CSSProperties {
        "--pad"?: string;
    }
}

const EDGE_MARGIN = 8;

const MAPPER_FLIP = Object.freeze({
    [TooltipPosition.TOP]: TooltipPosition.BOTTOM,
    [TooltipPosition.BOTTOM]: TooltipPosition.TOP,
    [TooltipPosition.LEFT]: TooltipPosition.RIGHT,
    [TooltipPosition.RIGHT]: TooltipPosition.LEFT,
} as const);

const MAPPER_NOOP = Object.freeze({
    [TooltipPosition.TOP]: TooltipPosition.TOP,
    [TooltipPosition.BOTTOM]: TooltipPosition.BOTTOM,
    [TooltipPosition.LEFT]: TooltipPosition.LEFT,
    [TooltipPosition.RIGHT]: TooltipPosition.RIGHT,
} as const);

const positionStyleMap: Record<TooltipPosition, string> = {
    [TooltipPosition.TOP]: styles.top,
    [TooltipPosition.BOTTOM]: styles.bottom,
    [TooltipPosition.LEFT]: styles.left,
    [TooltipPosition.RIGHT]: styles.right,
};


// keep in sync with scss file
const PAD = "var(--pad, 1rem)";


function makeArrowStyles(
    position: TooltipPosition,
    targetPos: RectOffset | undefined,
    baseRect: DOMRectReadOnly | undefined,
): CSSProperties {
    switch (position) {
        case TooltipPosition.TOP: {
            // FIXME: handle this case
            return {
                translate: "-50%",
            };
        }
        case TooltipPosition.BOTTOM: {
            let left: number | undefined;

            if (targetPos && baseRect) {
                left = Math.abs(baseRect.left - targetPos.left);
            }

            return {
                top: 0,
                rotate: "180deg",
                translate: `-50% calc(-100% + ${PAD})`,
                left,
            };
        }
        case TooltipPosition.LEFT: {
            // FIXME: handle this case
            return {
                left: "100%",
                rotate: "270deg",
                translate: `calc(-1 * ${PAD}) -50%`,
            };
        }
        case TooltipPosition.RIGHT: {
            // FIXME: handle this case
            return {
                left: PAD,
                rotate: "90deg",
                translate: "-100% -50%",
            };
        }
    }
}

interface MakeTooltipPositionStylesOptions {
    position: TooltipPosition;
    percentIn: SpringValue<number>;
    triggerHeight: SpringValue<number>;
    triggerWidth: SpringValue<number>;
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
                (percentIn, triggerHeight) => `calc(1rem + ${percentIn * triggerHeight}px)`,
            );

            return {
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
                paddingLeft,
                "--pad": paddingLeft,
            };
        }
    }
}

function getInitialPos(position: TooltipPosition, targetRect: DOMRectReadOnly) {
    let { top, left, width, height } = targetRect;

    switch (position) {
        case TooltipPosition.BOTTOM: {
            top += height;
            // fallthrough
        }
        case TooltipPosition.TOP: {
            left += width / 2;
            break;
        }
        case TooltipPosition.RIGHT: {
            left += width;
            // fallthrough
        }
        case TooltipPosition.LEFT: {
            top += height / 2;
            break;
        }
    }

    return {
        top,
        left,
    };
}

function computeRectClipOffsets(
    el: DOMRectReadOnly,
    bounds: DOMRectReadOnly,
    avoid: Pick<DOMRectReadOnly, "top" | "bottom" | "left" | "right">,
): [RectOffset, mapper: Record<TooltipPosition, TooltipPosition>] {
    let mapper: Record<TooltipPosition, TooltipPosition> = MAPPER_NOOP;

    if (rectFullyContainedBy(el, bounds)) {
        return [NO_OFFSET, mapper];
    }

    let top = 0;
    let left = 0;

    if (rectHeightCanBeContainedBy(el, bounds)) {
        // too far up
        if (el.top < bounds.top) {
            if (avoid.bottom > bounds.top) {
                mapper = MAPPER_FLIP;
            }
            top = Math.max(bounds.top, avoid.bottom) - el.top;
        // too far down
        } else if (el.bottom > bounds.bottom) {
            if (avoid.top < bounds.bottom) {
                mapper = MAPPER_FLIP;
            }
            top = Math.min(bounds.bottom, avoid.top) - el.bottom;
        }
    }

    if (rectWidthCanBeContainedBy(el, bounds)) {
        // too far left
        if (el.left < bounds.left) {
            if (avoid.right > bounds.left) {
                mapper = MAPPER_FLIP;
            }
            left = Math.max(bounds.left, avoid.right) - el.left;
        // too far right
        } else if (el.right > bounds.right) {
            if (avoid.left < bounds.right) {
                mapper = MAPPER_FLIP;
            }
            left = Math.min(bounds.right, avoid.left) - el.right;
        }
    }

    return [
        {
            top,
            left,
        },
        mapper,
    ];
}

function makeAvoidBounds(rect: DOMRectReadOnly, position: TooltipPosition): Pick<DOMRectReadOnly, "top" | "bottom" | "left" | "right"> {
    switch (position) {
        case TooltipPosition.TOP:
        case TooltipPosition.BOTTOM: {
            return {
                top: rect.top,
                bottom: rect.bottom,
                right: -Infinity,
                left: Infinity,
            };
        }
        case TooltipPosition.LEFT:
        case TooltipPosition.RIGHT: {
            return {
                left: rect.left,
                right: rect.right,
                top: -Infinity,
                bottom: Infinity,
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

interface TooltipArrowProps {
    targetElement: HTMLElement;
    contentFragment: RefObject<FragmentInstance | null>;
    position: TooltipPosition;
}

function TooltipArrow({ targetElement, contentFragment, position }: TooltipArrowProps) {
    const rect = useRect(targetElement);
    const contentRect = useFragmentRect(contentFragment);
    const targetPos = useMemo(() => rect && getInitialPos(position, rect), [position, rect]);

    return (
        <svg
            height="24"
            width="24"
            viewBox="0 0 24 24"
            className={styles.arrow}
            style={makeArrowStyles(position, targetPos, contentRect)}
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


interface PositionLayerReferenceProps {
    position: TooltipPosition;
    referenceElement: HTMLElement;
    /**
     * @default true
     */
    bumpIntoView: boolean;
    className: string;
    children: (actualPosition: TooltipPosition) => ReactNode;
}


function PositionLayerReference({
    position,
    referenceElement,
    bumpIntoView,
    className,
    children,
}: PositionLayerReferenceProps) {
    const [referenceRect] = useState(() => measureRect(referenceElement));
    const basePos = useMemo(() => getInitialPos(position, referenceRect), [referenceRect, position]);
    const avoidBounds = useMemo(() => makeAvoidBounds(referenceRect, position), [referenceRect, position]);
    const [offset, _setOffset] = useState<RectOffset>(NO_OFFSET);
    const [actualPosition, setActualPosition] = useState(position);
    // use a ref so we don't cause an infinite loop of renders
    const offsetRef = useRecent(offset);
    const finalPos = bumpIntoView ? mergeRectOffsets(basePos, offset) : basePos;
    const childrenRef = useRef<FragmentInstance>(null);
    const [childRect, setChildRect] = useState<DOMRectReadOnly | null>(null);
    const root = use(LayerContext).root ?? document.body;
    const _rootRect = useRect(root);
    const rootRect = useMemo(() => _rootRect && removeMarginFromRect(_rootRect, EDGE_MARGIN), [_rootRect]);

    const setOffset = useCallback((newOffset: RectOffset) => {
        if (bumpIntoView) {
            _setOffset((oldOffset) => {
                return compareRectOffsets(oldOffset, newOffset) ? oldOffset : newOffset;
            });
        } else {
            _setOffset(NO_OFFSET);
        }
    }, [bumpIntoView]);

    function recomputeOffset() {
        if (!rootRect || !bumpIntoView) {
            return;
        }

        const cRect = childRect ?? (childrenRef.current && measureFragmentRect(childrenRef.current));

        if (!cRect?.width || !cRect.height) {
            return;
        }

        // if we keep getting the child rect, then the offset will compound on itself
        // only use/update the child rect if we don't have an offset
        if (compareRectOffsets(offsetRef.current, NO_OFFSET)) {
            setChildRect(cRect);
        }

        // If we include the offset, then we will just flip back and forth between being in view and out of view
        const [newOffset, mapper] = computeRectClipOffsets(
            cRect,
            rootRect,
            avoidBounds,
        );

        setActualPosition(mapper[position]);


        setOffset(newOffset);
    }

    useLayoutEffect(recomputeOffset, [
        rootRect,
        childRect,
        position,
        offsetRef,
        referenceRect,
        avoidBounds,
        bumpIntoView,
        setOffset,
    ]);

    return (
        <div
            ref={useIntersection(recomputeOffset, { root })}
            style={finalPos}
            className={className}
        >
            {/* eslint-disable-next-line @eslint-react/no-useless-fragment -- Rel1cx/eslint-react#1567 */}
            <Fragment ref={childrenRef}>
                {children(actualPosition)}
            </Fragment>
        </div>
    );
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
     * if the tooltip is outside of the viewport, adjust the position
     *
     * @default true
     */
    bumpIntoView?: boolean;
}


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
    bumpIntoView = true,
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
    const [triggerEl, setTriggerEl] = useState<HTMLElement | null>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const triggerHeight = useSpringValue(0);
    const triggerWidth = useSpringValue(0);
    const [dep, updateSizeVar] = useForceUpdater();
    const tooltipContentFragmentRef = useRef<FragmentInstance>(null);

    useResizeObserver(triggerEl, updateSizeVar);

    useLayoutEffect(() => {
        if (triggerEl && containerRef.current) {
            const { width, height } = measureRect(triggerEl);

            if (!triggerHeight.hasAnimated) {
                triggerHeight.set(height);
                triggerWidth.set(width);
            } else {
                triggerHeight.start(height);
                triggerWidth.start(width);
            }
        }
    }, [dep, triggerEl, triggerHeight, triggerWidth]);

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
                {triggerEl && tooltipTransition(({ percentIn, ...styleProps }, show) => {
                    return show && (
                        <PositionLayerReference
                            position={position}
                            referenceElement={triggerEl}
                            className={cn(styles.positionWrapper, positionStyleMap[position], tooltipClassName)}
                            bumpIntoView={bumpIntoView}
                        >
                            {(actualPosition) => (
                                <animated.div
                                    style={{
                                        ...styleProps,
                                        ...makeTooltipPositionStyles({
                                            position: actualPosition,
                                            percentIn,
                                            triggerHeight,
                                            triggerWidth,
                                        }),
                                    }}
                                    className={styles.container}
                                >
                                    <Fragment ref={tooltipContentFragmentRef}>
                                        {noWrapper
                                            ? text
                                            : (
                                                <Box className={styles.box}>
                                                    <div className={styles.wrapper}>
                                                        {text}
                                                    </div>
                                                </Box>
                                            )}
                                    </Fragment>
                                    {noArrow || (
                                        <TooltipArrow
                                            position={actualPosition}
                                            targetElement={triggerEl}
                                            contentFragment={tooltipContentFragmentRef}
                                        />
                                    )}
                                </animated.div>
                            )}
                        </PositionLayerReference>
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

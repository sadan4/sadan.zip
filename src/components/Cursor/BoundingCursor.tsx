import { useCssFile } from "@/hooks/cssFile";
import { useCursorVisible } from "@/hooks/cursorVisible";
import { useEventHandler } from "@/hooks/eventListener";
import { useForceUpdater } from "@/hooks/forceUpdater";
import cn from "@/utils/cn";
import { clamp } from "@/utils/functional";
import useResizeObserver from "@react-hook/resize-observer";
import { FluidValue } from "@react-spring/shared";
import { animated, to, useSpring, useSpringValue } from "@react-spring/web";
import { useMove } from "@use-gesture/react";

import styles from "./BoundingCursor.module.scss";
import { useCursorContextStore } from "./cursorContextStore";
import hideFocusOutline from "./hideFocusOutline.scss?url";

import invariant from "invariant";
import { useCallback, useEffect } from "react";
import { createPortal } from "react-dom";

export type TFrameLength = {
    type: "static";
    length: number;
} | {
    type: "dynamic";
    min?: number;
    max?: number;
    factor?: number;
} | undefined;

export interface BoundingCursorProps {
    className?: string;
    frameLength?: TFrameLength;
    thickness?: number;
    unHoveredRadius?: number;
    hideOnExit?: boolean;
    borderFocusedItems?: boolean;
}

interface CursorPos {
    topLeftX: number | FluidValue<number>;
    topLeftY: number | FluidValue<number>;

    topRightX: number | FluidValue<number>;
    topRightY: number | FluidValue<number>;

    bottomRightX: number | FluidValue<number>;
    bottomRightY: number | FluidValue<number>;

    bottomLeftX: number | FluidValue<number>;
    bottomLeftY: number | FluidValue<number>;

    height: number | FluidValue<number>;
    width: number | FluidValue<number>;
}

interface BarLengths {
    xLength: number | FluidValue<number>;
    yLength: number | FluidValue<number>;
}

function RemoveFocusOutline() {
    useCssFile(hideFocusOutline);

    return null;
}

export default function BoundingCursor({
    className,
    frameLength = {
        type: "static",
        length: 15,
    },
    thickness: _thickness = 5,
    unHoveredRadius: _unHoveredRadius = 15,
    borderFocusedItems = false,
}: BoundingCursorProps) {
    const element = useCursorContextStore((state) => {
        if (borderFocusedItems && state.focusedElement) {
            return state.focusedElement;
        }
        return state.clickableElement;
    });

    const cursorVisible = useCursorVisible(false);
    const mouseX = useSpringValue(useCursorContextStore.getState().lastMousePos.x || window.innerWidth / 2);
    const mouseY = useSpringValue(useCursorContextStore.getState().lastMousePos.y || window.innerHeight / 2);

    useMove(({ xy: [mx, my] }) => {
        mouseX.set(mx);
        mouseY.set(my);
    }, {
        target: document,
    });

    const makeCursorPos = useCallback((
        element: Element | null,
        _unHoveredRadius: number | FluidValue<number>,
        mouseX: number | FluidValue<number>,
        mouseY: number | FluidValue<number>,
    ): CursorPos => {
        const isHovering = element !== null;

        const { width, height, x, y } = (() => {
            const elemSize = element?.getBoundingClientRect();

            return isHovering && elemSize
                ? elemSize
                : {
                    height: to(_unHoveredRadius, (uhr) => 2 * uhr),
                    width: to(_unHoveredRadius, (uhr) => 2 * uhr),
                    x: to([mouseX, _unHoveredRadius], (mx, uhr) => mx - uhr),
                    y: to([mouseY, _unHoveredRadius], (my, uhr) => my - uhr),
                };
        })();

        return {
            topLeftX: x,
            topLeftY: y,
            topRightX: to([x, width], (x, width) => x + width),
            topRightY: y,
            bottomLeftX: x,
            bottomLeftY: to([y, height], (y, height) => y + height),
            bottomRightX: to([x, width], (x, width) => x + width),
            bottomRightY: to([y, height], (y, height) => y + height),
            width,
            height,
        };
    }, []);

    const makeBarLengths = useCallback(({ width, height }: Pick<CursorPos, "width" | "height">): BarLengths => {
        switch (frameLength.type) {
            case "static": {
                const { length } = frameLength;

                return {
                    xLength: length,
                    yLength: length,
                };
            }
            case "dynamic": {
                const { min = 15, max = 100, factor = 1 / 8 } = frameLength;

                return {
                    xLength: to(width, (width) => Math.round(clamp(min, max, width * factor))),
                    yLength: to(height, (height) => Math.round(clamp(min, max, height * factor))),
                };
            }
            default: {
                invariant(false, "invalid frame length type");
            }
        }
    }, [frameLength]);

    const [
        {
            topLeftX,
            topLeftY,

            topRightX,
            topRightY,

            bottomRightX,
            bottomRightY,

            bottomLeftX,
            bottomLeftY,

            xLength,
            yLength,

            thickness,
            unHoveredRadius,
        },
        api,
    ] = useSpring(() => {
        const pos = makeCursorPos(element, _unHoveredRadius, mouseX, mouseY);
        const barLen = makeBarLengths(pos);

        return {
            ...pos,
            ...barLen,
            thickness: _thickness,
            unHoveredRadius: _unHoveredRadius,

            config: {
                tension: 300,
                friction: 20,
            },
        } as const;
    });

    const [dep, updateElementSize] = useForceUpdater();

    // update when we're hovering an element
    useEffect(() => {
        if (element) {
            const pos = makeCursorPos(element, unHoveredRadius, mouseX, mouseY);
            const barLen = makeBarLengths(pos);

            api.start({
                ...pos,
                ...barLen,
            });
        }
    }, [api, element, makeBarLengths, makeCursorPos, mouseX, mouseY, unHoveredRadius, dep]);

    useResizeObserver(element, updateElementSize);
    useEventHandler("scroll", updateElementSize);

    const opacity = useSpringValue(0);

    useEffect(() => {
        if (cursorVisible) {
            opacity.start(1);
        } else {
            opacity.start(0);
        }
    }, [cursorVisible, opacity]);

    return (
        <>
            {
                createPortal(

                    (
                        <>
                            {borderFocusedItems && <RemoveFocusOutline />}
                            <animated.div
                                style={{
                                    opacity,
                                }}
                            >
                                <div
                                    className={cn(className, "pointer-events-none absolute top-0 left-0", styles.boundingCursor)}
                                >
                                    {/* Top-left corner */}
                                    <animated.div
                                        style={{
                                            left: to([topLeftX, thickness], (tlx, thick) => tlx - thick),
                                            top: to([topLeftY, thickness], (tly, thick) => tly - thick),
                                            width: to([xLength, thickness], (xLen, thick) => xLen + thick),
                                            height: thickness,
                                        }}
                                    />
                                    <animated.div
                                        style={{
                                            left: to([topLeftX, thickness], (tlx, thick) => tlx - thick),
                                            top: to([topLeftY, thickness], (tly, thick) => tly - thick),
                                            width: thickness,
                                            height: to([yLength, thickness], (yLen, thick) => yLen + thick),
                                        }}
                                    />
                                    {/* Top-right corner */}
                                    <animated.div
                                        style={{
                                            left: to([topRightX, xLength], (tlx, xLen) => tlx - xLen),
                                            top: to([topRightY, thickness], (tly, thick) => tly - thick),
                                            width: to([xLength, thickness], (len, thick) => len + thick),
                                            height: thickness,
                                        }}
                                    />
                                    <animated.div
                                        style={{
                                            left: topRightX,
                                            top: to([topRightY, thickness], (val, thick) => val - thick),
                                            width: thickness,
                                            height: to([yLength, thickness], (len, thick) => len + thick),
                                        }}
                                    />
                                    {/* Bottom-left corner */}
                                    <animated.div
                                        style={{
                                            left: to([bottomLeftX, thickness], (val, thick) => val - thick),
                                            top: to([bottomLeftY, yLength], (val, len) => val - len),
                                            width: thickness,
                                            height: to([yLength, thickness], (len, thick) => len + thick),
                                        }}
                                    />
                                    <animated.div
                                        style={{
                                            left: to([bottomLeftX, thickness], (val, thick) => val - thick),
                                            top: bottomLeftY,
                                            width: to([xLength, thickness], (len, thick) => len + thick),
                                            height: thickness,
                                        }}
                                    />
                                    {/* Bottom-right corner */}
                                    <animated.div
                                        style={{
                                            left: bottomRightX,
                                            top: to([bottomRightY, yLength], (val, len) => val - len),
                                            width: thickness,
                                            height: to([yLength, thickness], (len, thick) => len + thick),
                                        }}
                                    />
                                    <animated.div
                                        style={{
                                            left: to([bottomRightX, xLength], (val, len) => val - len),
                                            top: bottomRightY,
                                            width: to([xLength, thickness], (len, thick) => len + thick),
                                            height: thickness,
                                        }}
                                    />
                                </div>
                            </animated.div>
                        </>
                    ),
                    document.body,
                )
            }
        </>
    );
}

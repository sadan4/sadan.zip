import { useCursorVisible } from "@/hooks/cursorVisible";
import { useForceUpdater } from "@/hooks/forceUpdater";
import cn from "@/utils/cn";
import useResizeObserver from "@react-hook/resize-observer";
import { FluidValue } from "@react-spring/shared";
import { animated, to, useSpring, useSpringValue } from "@react-spring/web";
import { useMove } from "@use-gesture/react";

import styles from "./BoundingCursor.module.css";
import { CursorClickableContext, lastPos } from "./context";

import invariant from "invariant";
import _ from "lodash";
import { useCallback, useContext, useEffect } from "react";
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


export default function BoundingCursor({
    className,
    frameLength = {
        type: "static",
        length: 15,
    },
    thickness: _thickness = 5,
    unHoveredRadius: _unHoveredRadius = 15,
}: BoundingCursorProps) {
    const element = useContext(CursorClickableContext);
    const cursorVisible = useCursorVisible(false);
    const mouseX = useSpringValue(lastPos.x || window.innerWidth / 2);
    const mouseY = useSpringValue(lastPos.y || window.innerHeight / 2);

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
                    xLength: to(width, (width) => _.chain(width * factor)
                        .clamp(min, max)
                        .round()
                        .valueOf()),
                    yLength: to(height, (height) => _.chain(height * factor)
                        .clamp(min, max)
                        .round()
                        .valueOf()),
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
                        <animated.div
                            style={{
                                opacity,
                            }}
                        >
                            <div
                                className={cn(className, "absolute top-0 left-0 pointer-events-none", styles.boundingCursor)}
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
                    ),
                    document.body,
                )
            }
        </>
    );
}

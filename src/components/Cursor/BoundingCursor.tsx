import { useCursorVisible } from "@/hooks/cursorVisible";
import cn from "@/utils/cn";
import type { Coord } from "@/utils/types";

import styles from "./BoundingCursor.module.css";
import { CursorClickableContext, CursorPosContext } from "./context";

import invariant from "invariant";
import _ from "lodash";
import { useContext } from "react";
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

export default function BoundingCursor({
    className,
    frameLength = {
        type: "static",
        length: 15,
    },
    thickness = 5,
    unHoveredRadius = 15,
}: BoundingCursorProps) {
    const element = useContext(CursorClickableContext);
    const { x: mouseX, y: mouseY } = useContext(CursorPosContext);
    const isHovering = element !== null;
    const onScreen = useCursorVisible();

    const { topLeft, topRight, bottomLeft, bottomRight, width, height } = (() => {
        const { width, height, x, y } = isHovering
            ? element.getBoundingClientRect()
            : {
                height: 2 * unHoveredRadius,
                width: 2 * unHoveredRadius,
                x: mouseX - unHoveredRadius,
                y: mouseY - unHoveredRadius,
            };

        return {
            topLeft: {
                x,
                y,
            } satisfies Coord,
            topRight: {
                x: x + width,
                y,
            } satisfies Coord,
            bottomLeft: {
                x,
                y: y + height,
            } satisfies Coord,
            bottomRight: {
                x: x + width,
                y: y + height,
            } satisfies Coord,
            width,
            height,
        };
    })();

    const { xLength, yLength } = (() => {
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
                    xLength: _.chain(width! * factor)
                        .clamp(min, max)
                        .round()
                        .value(),
                    yLength: _.chain(height! * factor)
                        .clamp(min, max)
                        .round()
                        .value(),
                };
            }
            default: {
                invariant(false, "invalid frame length type");
            }
        }
    })();

    return (
        <>
            {
                createPortal(

                    onScreen && (
                        <div
                            className={cn(className, "absolute top-0 left-0 pointer-events-none", styles.boundingCursor)}
                        >
                            {/* Top-left corner */}
                            <div
                                style={{
                                    left: topLeft.x - thickness,
                                    top: topLeft.y - thickness,
                                    width: xLength + thickness,
                                    height: thickness,
                                }}
                            />
                            <div
                                style={{
                                    left: topLeft.x - thickness,
                                    top: topLeft.y - thickness,
                                    width: thickness,
                                    height: yLength + thickness,
                                }}
                            />
                            {/* Top-right corner */}
                            <div
                                style={{
                                    left: topRight.x - xLength,
                                    top: topRight.y - thickness,
                                    width: xLength + thickness,
                                    height: thickness,
                                }}
                            />
                            <div
                                style={{
                                    left: topRight.x,
                                    top: topRight.y - thickness,
                                    width: thickness,
                                    height: yLength + thickness,
                                }}
                            />
                            {/* Bottom-left corner */}
                            <div
                                style={{
                                    left: bottomLeft.x - thickness,
                                    top: bottomLeft.y - yLength,
                                    width: thickness,
                                    height: yLength + thickness,
                                }}
                            />
                            <div
                                style={{
                                    left: bottomLeft.x - thickness,
                                    top: bottomLeft.y,
                                    width: xLength + thickness,
                                    height: thickness,
                                }}
                            />
                            {/* Bottom-right corner */}
                            <div
                                style={{
                                    left: bottomRight.x,
                                    top: bottomRight.y - yLength,
                                    width: thickness,
                                    height: yLength + thickness,
                                }}
                            />
                            <div
                                style={{
                                    left: bottomRight.x - xLength,
                                    top: bottomRight.y,
                                    width: xLength + thickness,
                                    height: thickness,
                                }}
                            />
                        </div>
                    ),
                    document.body,
                )
            }
        </>
    );
}

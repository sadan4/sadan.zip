import { useSize } from "@/hooks/size";
import { single } from "@/utils/array";
import { parseCSSValue, PercentReference } from "@/utils/dom";
import { ellipseCircumference } from "@/utils/math";
import toCSS from "@/utils/toCSS";
import useResizeObserver from "@react-hook/resize-observer";
import { animated, useSpringValue } from "@react-spring/web";

import styles from "./any.module.scss";
import { borderRadiusToPath } from "./guh";

import { type PropsWithChildren, useCallback, useEffect, useRef, useState } from "react";

export interface BolderHoldCircularProps extends PropsWithChildren {
    onHold?: () => void;
}

interface BorderCalcContext {
    element: Element;
    width: number;
    height: number;
}

function normalizeRadius(radius: string, { element, width, height }: BorderCalcContext): [a: number, b: number] {
    let a: string,
        b = a = radius;

    if (radius.includes(" ")) {
        [a, b] = radius.split(" ");
    }

    const parsedA: number = Math.min(parseCSSValue(a, element, PercentReference.WIDTH), width / 2);
    const parsedB: number = Math.min(parseCSSValue(b, element, PercentReference.HEIGHT), height / 2);

    return [parsedA, parsedB];
}

function calcRadiusDelta(radius: string, ctx: BorderCalcContext): number {
    const [a, b] = normalizeRadius(radius, ctx);
    const curveLen = ellipseCircumference(a, b) / 4;
    const delta = curveLen - (a + b);

    return delta;
}

function calculateBorderLength(element: Element): number {
    const { width, height } = element.getBoundingClientRect();
    let rectLength = 2 * (width + height);
    const style = getComputedStyle(element);
    const topLeft = style.borderTopLeftRadius;
    const topRight = style.borderTopRightRadius;
    const bottomRight = style.borderBottomRightRadius;
    const bottomLeft = style.borderBottomLeftRadius;

    const context: BorderCalcContext = {
        element,
        width,
        height,
    };

    rectLength += calcRadiusDelta(topLeft, context);
    rectLength += calcRadiusDelta(topRight, context);
    rectLength += calcRadiusDelta(bottomRight, context);
    rectLength += calcRadiusDelta(bottomLeft, context);

    return rectLength;
}

export default function BorderHoldCircular({ children, onHold }: BolderHoldCircularProps) {
    const wrapperRef = useRef<HTMLDivElement>(null);
    const borderRef = useRef<SVGPathElement>(null);
    const [borderLen, setBorderLen] = useState(-1);

    const { width, height } = useSize(() => wrapperRef.current) ?? {
        width: 0,
        height: 0,
    };

    const bgWidth = width * (1 + (1 / 15));
    const bgHeight = height * (1 + (1 / 15));

    const updateBorderLength = useCallback(() => {
        if (wrapperRef.current) {
            const child = single(wrapperRef.current.children);

            setBorderLen(calculateBorderLength(child));
            if (borderRef.current) {
                const computed = getComputedStyle(child);

                borderRef.current.setAttribute("d", borderRadiusToPath({
                    width,
                    height,
                    top: 0,
                    left: 0,
                }, computed));
            }
        }
    }, [height, width]);

    useResizeObserver(wrapperRef, updateBorderLength);

    useEffect(updateBorderLength, [updateBorderLength]);

    const opacity = useSpringValue(0);
    const dispatched = useRef(false);

    const progress = useSpringValue(0, {
        config: {
            mass: 5,
            friction: 110,
        },
        onChange(_foo) {
            // https://github.com/pmndrs/react-spring/issues/2183
            const foo: number = typeof _foo === "number"
                ? _foo
                : _foo.value;

            if (foo > 98 && progress.goal === 100 && !dispatched.current) {
                dispatched.current = true;
                onHold?.();
            } else if (foo < 2 && progress.goal === 0) {
                opacity.start(0);
                dispatched.current = false;
            }
        },
    });

    const startAnimation = useCallback(() => {
        progress.start(100, {
            config: {
                friction: 110,
            },
        });
        opacity.start(1);
    }, [opacity, progress]);

    const stopAnimation = useCallback(() => {
        progress.start(0, {
            config: {
                friction: 55,
            },
        });
    }, [progress]);

    return (
        <div
            className="relative"
            onPointerDown={startAnimation}
            onContextMenu={(e) => {
                // it's a pointer event, react is stupid https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event#browser_compatibility
                if ((e.nativeEvent as PointerEvent).pointerType !== "mouse") {
                    e.preventDefault();
                }
            }}
            onPointerUp={stopAnimation}
            onPointerLeave={stopAnimation}
        >
            <div
                className="contents"
                ref={wrapperRef}
            >
                {children}
            </div>
            borderLen: {borderLen}
            <animated.svg
                className={styles.rectBorder}
                style={{
                    width: toCSS.px(bgWidth),
                    height: toCSS.px(bgHeight),
                    ["--border-hold-progress" as any]: progress,
                    ["--border-len" as any]: borderLen,
                    opacity,
                }}
            >
                <path
                    ref={borderRef}
                    style={{
                        strokeWidth: (bgWidth + bgHeight) / 20,
                    }}
                />
            </animated.svg>
        </div>
    );
}

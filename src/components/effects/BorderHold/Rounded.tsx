import { useSize } from "@/hooks/size";
import { single } from "@/utils/array";
import { parseCSSValue, PercentReference } from "@/utils/dom";
import { ellipseCircumference } from "@/utils/math";
import useResizeObserver from "@react-hook/resize-observer";
import { animated, useSpringValue } from "@react-spring/web";

import styles from "./rounded.module.scss";

import { type PropsWithChildren, type RefObject, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";

export interface BorderHoldHandle {
    recalculateBorder(): void;
}

export interface BorderHoldCircularProps extends PropsWithChildren {
    onHold?: () => void;
    ref?: RefObject<BorderHoldHandle | null>;
}


function calculateBorderLength(element: Element): [length: number, path: string] {
    const { width, height } = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    const [topLeftA, topLeftB] = normalizeRadius(style.borderTopLeftRadius);
    const [topRightA, topRightB] = normalizeRadius(style.borderTopRightRadius);
    const [bottomRightA, bottomRightB] = normalizeRadius(style.borderBottomRightRadius);
    const [bottomLeftA, bottomLeftB] = normalizeRadius(style.borderBottomLeftRadius);
    let isSquare = true;
    let rectLength = 2 * (width + height);

    rectLength += calcRadiusDelta(topLeftA, topLeftB);
    rectLength += calcRadiusDelta(topRightA, topRightB);
    rectLength += calcRadiusDelta(bottomRightA, bottomRightB);
    rectLength += calcRadiusDelta(bottomLeftA, bottomLeftB);

    const path = makePath();

    return [rectLength, path];

    function makePath(): string {
        if (isSquare) {
            return makeSquarePath();
        }

        return `
            M ${width / 2} 0
            H ${width - topRightA}
            A ${topRightA} ${topRightB} 0 0 1 ${width} ${topRightB}
            V ${height - bottomRightB}
            A ${bottomRightA} ${bottomRightB} 0 0 1 ${width - bottomRightA} ${height}
            H ${bottomLeftA}
            A ${bottomLeftA} ${bottomLeftB} 0 0 1 0 ${height - bottomLeftB}
            V ${topLeftB}
            A ${topLeftA} ${topLeftB} 0 0 1 ${topLeftA} 0
            Z
        `;
    }

    function makeSquarePath(): string {
        return `
            M ${width / 2} 0
            H ${width}
            V ${height}
            H 0
            V 0
            Z
        `;
    }

    function calcRadiusDelta(a, b): number {
        if (!a && !b) {
            return 0;
        }
        isSquare = false;

        const curveLen = ellipseCircumference(a, b) / 4;
        const delta = curveLen - (a + b);

        return delta;
    }

    function normalizeRadius(radius: string): [a: number, b: number] {
        let a: string,
            b = a = radius;

        if (radius.includes(" ")) {
            [a, b] = radius.split(" ");
        }

        const parsedA: number = Math.min(parseCSSValue(a, element, PercentReference.WIDTH), width / 2);
        const parsedB: number = Math.min(parseCSSValue(b, element, PercentReference.HEIGHT), height / 2);

        return [parsedA, parsedB];
    }
}

export default function BorderHoldRounded({ children, onHold, ref }: BorderHoldCircularProps) {
    const wrapperRef = useRef<HTMLDivElement>(null);
    const borderRef = useRef<SVGPathElement>(null);
    const [borderLen, setBorderLen] = useState(-1);

    const { width, height } = useSize(() => wrapperRef.current) ?? {
        width: 0,
        height: 0,
    };

    const updateBorderLength = useCallback(() => {
        if (wrapperRef.current) {
            const child = single(wrapperRef.current.children);
            const [length, path] = calculateBorderLength(child);

            setBorderLen(length);
            if (borderRef.current) {
                borderRef.current.setAttribute("d", path);
            }
        }
    }, []);

    useImperativeHandle(ref, () => ({
        recalculateBorder: updateBorderLength,
    }), [updateBorderLength]);

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
        <svg
            className={styles.roundedBorder}
            style={{
                width,
                height,
            }}
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
            <animated.path
                ref={borderRef}
                style={{
                    // mean(width, height) / 10
                    strokeWidth: (width + height) / 20,
                    strokeDasharray: progress.to((progress) => {
                        const curLen = (borderLen * progress) / 100;

                        return `${curLen} ${borderLen - curLen}`;
                    }),
                    opacity,
                }}
            />
            <foreignObject
                style={{
                    width,
                    height,
                }}
            >
                <div
                    className="contents"
                    ref={wrapperRef}
                >
                    {children}
                </div>
            </foreignObject>
        </svg>
    );
}

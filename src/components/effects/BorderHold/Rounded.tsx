import { useRect } from "@/hooks/rect";
import { useResizeObserver } from "@/hooks/resizeObserver";
import { single } from "@/utils/array";
import { makeBorderPath } from "@/utils/dom/path";
import { animated, SpringRef, useSpring } from "@react-spring/web";

import { type BaseBorderHoldProps, borderHoldAnimConfig } from "./common";
import styles from "./rounded.module.scss";

import { type Ref, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";

export interface BorderHoldSpring {
    progress: number;
    opacity: number;
}

export interface BorderHoldHandle {
    recalculateBorder(): void;
    reactSpringApi: SpringRef<BorderHoldSpring>;
    onStopHold(): void;
}

export interface BorderHoldRoundedProps extends BaseBorderHoldProps {
    ref?: Ref<BorderHoldHandle | null>;
    onPointerDown?: () => void;
}

export function BorderHoldRounded({ children, onHold, onPointerDown, ref }: BorderHoldRoundedProps) {
    const [wrapper, setWrapper] = useState<HTMLDivElement | null>(null);
    const borderRef = useRef<SVGPathElement>(null);
    const maskRef = useRef<SVGPathElement>(null);
    const [borderLen, setBorderLen] = useState(-1);

    const { width, height } = useRect(wrapper) ?? {
        width: 1,
        height: 1,
    };

    const updateBorderLength = useCallback(() => {
        if (wrapper) {
            const child = single(wrapper.children);
            const [length, path] = makeBorderPath(child);

            setBorderLen(length);
            borderRef.current?.setAttribute("d", path);
            maskRef.current?.setAttribute("d", path);
        }
    }, [wrapper]);

    useResizeObserver(wrapper, updateBorderLength);

    useEffect(updateBorderLength, [updateBorderLength]);

    const dispatchedRef = useRef(false);

    const [{ progress, opacity }, api] = useSpring(() => ({
        from: {
            progress: 0,
            opacity: 0,
        },
    }));

    const onStartHold = useCallback(() => {
        api.start({
            async to(next) {
                await next({
                    progress: 100,
                    opacity: 1,
                    onChange(progress) {
                        // bug in react-spring types
                        const value = progress.value.progress as number;

                        if (!progress.cancelled && !dispatchedRef.current && value >= 98) {
                            dispatchedRef.current = true;
                            onHold?.();
                        }
                    },
                });
            },
            config: borderHoldAnimConfig(true),
        });
    }, [api, onHold]);

    const onStopHold = useCallback(() => {
        api.start({
            async to(next) {
                await next({
                    progress: 0,
                    onChange(progress) {
                        if (!progress.cancelled && (progress.value.progress as number) <= 5) {
                            // react spring doesn't like this, but it works
                            next({
                                opacity: 0,
                            }).catch(() => {});
                            dispatchedRef.current = false;
                        }
                    },
                });
            },
            config: borderHoldAnimConfig(false),
        });
    }, [api]);

    useImperativeHandle(ref, () => ({
        recalculateBorder: updateBorderLength,
        reactSpringApi: api,
        onStopHold,
    }), [api, onStopHold, updateBorderLength]);


    // mean(width, height) / 10
    const strokeWidth = (width + height) / 20;

    return (
        <div>
            <svg
                className={styles.roundedBorder}
                style={{
                    width,
                    height,
                }}
                onPointerDown={() => {
                    onPointerDown?.();
                    onStartHold();
                }}
                onContextMenu={(e) => {
                // it's a pointer event, react is stupid https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event#browser_compatibility
                    if ((e.nativeEvent as PointerEvent).pointerType !== "mouse") {
                        e.preventDefault();
                    }
                }}
                onPointerUp={() => onStopHold()}
                onPointerLeave={() => onStopHold()}
            >
                <animated.path
                    ref={borderRef}
                    style={{
                        strokeWidth,
                        strokeDasharray: progress.to((progress) => {
                            const curLen = (borderLen * progress) / 100;

                            return `${curLen} ${borderLen - curLen}`;
                        }),
                        opacity,
                    }}
                    mask="url(#m)"
                />
                <mask
                    maskContentUnits="userSpaceOnUse"
                    id="m"
                >
                    <rect
                        x={-strokeWidth}
                        y={-strokeWidth}
                        width={width + (strokeWidth * 2)}
                        height={height + (strokeWidth * 2)}
                    />
                    <path ref={maskRef} />
                </mask>
            </svg>
            <div ref={setWrapper}>
                {children}
            </div>
        </div>
    );
}

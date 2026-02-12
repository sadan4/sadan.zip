import { useRect } from "@/hooks/rect";
import { useResizeObserver } from "@/hooks/resizeObserver";
import { single } from "@/utils/array";
import { makeBorderPath } from "@/utils/dom";
import { animated } from "@react-spring/web";

import { type BaseBorderHoldProps, useBorderHoldAnim } from "./common";
import styles from "./rounded.module.scss";

import { type RefObject, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";

export interface BorderHoldHandle {
    recalculateBorder(): void;
}

export interface BorderHoldCircularProps extends BaseBorderHoldProps {
    ref?: RefObject<BorderHoldHandle | null>;
}

export function BorderHoldRounded({ children, onHold, ref }: BorderHoldCircularProps) {
    const [wrapper, setWrapper] = useState<HTMLDivElement | null>(null);
    const [held, setHeld] = useState(false);
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

    useImperativeHandle(ref, () => ({
        recalculateBorder: updateBorderLength,
    }), [updateBorderLength]);

    useResizeObserver(wrapper, updateBorderLength);

    useEffect(updateBorderLength, [updateBorderLength]);

    const { progress, opacity } = useBorderHoldAnim({
        held,
        onHold,
    });

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
                onPointerDown={() => setHeld(true)}
                onContextMenu={(e) => {
                // it's a pointer event, react is stupid https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event#browser_compatibility
                    if ((e.nativeEvent as PointerEvent).pointerType !== "mouse") {
                        e.preventDefault();
                    }
                }}
                onPointerUp={() => setHeld(false)}
                onPointerLeave={() => setHeld(false)}
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

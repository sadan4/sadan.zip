import { useRect } from "@/hooks/rect";
import { useResizeObserver } from "@/hooks/resizeObserver";
import { single } from "@/utils/array";
import cn from "@/utils/cn";
import { makeBorderPath } from "@/utils/dom/path";
import type { TOmit } from "@/utils/types";
import { animated, type SpringValue, to } from "@react-spring/web";

import * as styles from "./styles.module.scss";

import { type ComponentProps, type ComponentPropsWithoutRef, type Ref, useCallback, useEffect, useId, useImperativeHandle, useRef, useState } from "react";

type AnimatedPath = typeof animated.path;

export interface BorderProgressProps extends ComponentPropsWithoutRef<"svg"> {
    /**
     * higher number, thinner border.
     * 
     * min value: 1
     *
     * @default 10
     */
    widthCoefficient?: number;
    progress: SpringValue<number> | number;
    pathStyle?: TOmit<ComponentProps<AnimatedPath>["style"] & {}, "strokeDasharray" | "strokeWidth">;
    ref?: Ref<BorderProgress.Handle>;
}

export namespace BorderProgress {
    export interface Handle {
        recalculateBorder(): void;
    }
}

export function BorderProgress({
    children,
    widthCoefficient = 10,
    progress,
    pathStyle,
    ref,
    ...props
}: BorderProgressProps) {
    const [wrapper, setWrapper] = useState<HTMLDivElement | null>(null);
    const borderRef = useRef<SVGPathElement>(null);
    const maskRef = useRef<SVGPathElement>(null);
    const [borderLen, setBorderLen] = useState(-1);
    const maskId = useId();
    const { width = 1, height = 1 } = useRect(wrapper) ?? {};
    // mean(width, height) / widthCoefficient
    const strokeWidth = (width + height) / (widthCoefficient * 2);

    const updateBorderLength = useCallback(() => {
        if (wrapper) {
            const child = single(wrapper.children);
            const [length, path] = makeBorderPath(child);

            // oxlint-disable-next-line react/react-compiler
            setBorderLen(length);
            borderRef.current?.setAttribute("d", path);
            maskRef.current?.setAttribute("d", path);
        }
    }, [wrapper]);

    useResizeObserver(wrapper, updateBorderLength);

    useEffect(updateBorderLength, [updateBorderLength]);

    useImperativeHandle(ref, () => ({
        recalculateBorder: updateBorderLength,
    }), [updateBorderLength]);

    return (
        <div className="relative">
            <svg
                {...props}
                style={{
                    ...props.style,
                    width,
                    height,
                }}
                className={cn(styles.borderProgress, props.className)}
            >
                <animated.path
                    ref={borderRef}
                    style={{
                        strokeWidth,
                        strokeDasharray: to(progress, (progress) => {
                            const curLen = (borderLen * progress) / 100;

                            return `${curLen} ${borderLen - curLen}`;
                        }),
                        ...pathStyle,
                    }}
                    mask={`url(#${maskId})`}
                />
                <mask
                    maskContentUnits="userSpaceOnUse"
                    id={maskId}
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

import { useEventHandler } from "@/hooks/eventListener";
import { useRecent } from "@/hooks/recent";
import cn from "@/utils/cn";
import { assert } from "@/utils/error";
import { clamp } from "@/utils/math";

import type { ResizeHandleProps } from ".";
import { Direction, getBounds } from "./bounds";
import * as styles from "./styles.module.scss";

import { useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";

export interface VerticalResizeHandleProps extends ResizeHandleProps {
}

declare module "react" {
    interface CSSProperties {
        /**
         * 0-1
         */
        "--initial-drag-offset"?: number;
        // not passed as a style prop directly, no need to type it
        // /**
        //  * percent string
        //  */
        // "--drag-offset"?: string;
    }
}

export function Vertical({
    className,
    style,
    boundingElementRef,
    onDoubleClick,
    onResize,
    onResizeFinish,
    initialPosition = 0.5,
    onReset,
    ref,
    minPosition = 0,
    maxPosition = 1,
    ...props
}: VerticalResizeHandleProps) {
    assert(initialPosition >= 0 && initialPosition <= 1 && minPosition < initialPosition && initialPosition < maxPosition, "Invalid initial position");
    assert(minPosition >= 0 && maxPosition <= 1 && minPosition < maxPosition, "Invalid min/max position");

    const controllerRef = useRef(new AbortController());
    const handleRef = useRef<HTMLDivElement>(null);
    const [dragging, setDragging] = useState(false);
    const onResizeHandler = useRecent(onResize);
    const onResizeFinishHandler = useRecent(onResizeFinish);

    const dispatchResize = useCallback((final = false) => {
        if (handleRef.current && boundingElementRef.current) {
            const { toPercentage } = getBounds(Direction.VERTICAL, boundingElementRef.current);
            const { left: handleLeft, width: handleWidth } = handleRef.current.getBoundingClientRect();
            const num = toPercentage(handleLeft + (handleWidth / 2)) * 100;

            if (final) {
                onResizeFinishHandler.current?.(num);
                controllerRef.current.abort();
                controllerRef.current = new AbortController();
            } else {
                onResizeHandler.current?.(num);
            }
        }
    }, [boundingElementRef, onResizeFinishHandler, onResizeHandler]);

    const dispatchReset = useCallback(() => {
        onReset?.();
    }, [onReset]);

    const stopDragging = useCallback(() => {
        setDragging(false);
        dispatchResize(true);
    }, [dispatchResize]);

    const reset = useCallback(() => {
        handleRef.current?.style.removeProperty("--drag-offset");
        stopDragging();
        dispatchResize();
        dispatchReset();
    }, [dispatchReset, dispatchResize, stopDragging]);

    useImperativeHandle(ref, () => ({
        setCurrentPos(percent, shouldDispatch = false) {
            if (handleRef.current) {
                handleRef.current.style.setProperty("--drag-offset", `${percent / 100}`);
                if (shouldDispatch) {
                    dispatchResize();
                    dispatchResize(true);
                }
            }
        },
        reset() {
            reset();
        },
    }), [dispatchResize, reset]);

    useEventHandler("pointerup", stopDragging, undefined, {
        passive: true,
    });

    useEventHandler("pointermove", (e) => {
        if (!(handleRef.current && dragging)) {
            return;
        }

        const { toPercentage, clampToBounds } = getBounds(Direction.VERTICAL, boundingElementRef.current);
        const percent = clamp(minPosition, maxPosition, toPercentage(clampToBounds(e.clientX)));

        handleRef.current.style.setProperty("--drag-offset", `${percent}`);
        dispatchResize();
    }, undefined, {
        passive: true,
    });

    useEffect(() => {
        const controller = new AbortController();

        if (boundingElementRef.current) {
            boundingElementRef.current.addEventListener("pointerleave", stopDragging, {
                passive: true,
                signal: controller.signal,
            });
        } else {
            console.warn("Bounding element not found");
        }

        return () => {
            controller.abort();
        };
    }, [boundingElementRef, stopDragging]);

    useEffect(() => {
        dispatchResize(true);
        dispatchResize(false);
    }, [dispatchResize]);


    return (
        <div
            className={cn(styles.verticalHandle, dragging && styles.dragging, className)}
            style={{
                "--initial-drag-offset": initialPosition,
                ...style,
            }}
            ref={handleRef}
            onDoubleClick={(e) => {
                reset();
                onDoubleClick?.(e);
            }}
            {...props}
            onPointerDown={(e) => {
                if (e.isPrimary) {
                    setDragging(true);
                    // prevent selection while we are dragging, this controller will be aborted when we finish resizing
                    window.addEventListener("selectstart", (e) => e.preventDefault(), { signal: controllerRef.current.signal });
                }
                props.onPointerDown?.(e);
            }}
        />
    );
}

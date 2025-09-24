import { useEventHandler } from "@/hooks/eventListener";
import cn from "@/utils/cn";

import type { ResizeHandleProps } from ".";
import { Direction, getBounds } from "./bounds";
import styles from "./styles.module.scss";

import invariant from "invariant";
import { useCallback, useEffect, useRef, useState } from "react";

export interface VerticalResizeHandleProps extends ResizeHandleProps {
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
    ...props
}: VerticalResizeHandleProps) {
    invariant(initialPosition >= 0 && initialPosition <= 1, "Invalid initial position");

    const handleRef = useRef<HTMLDivElement>(null);
    const [dragging, setDragging] = useState(false);

    const dispatchResize = useCallback((final = false) => {
        if (handleRef.current && boundingElementRef.current) {
            const { toPercentage } = getBounds(Direction.VERTICAL, boundingElementRef.current);
            const { left: handleLeft, width: handleWidth } = handleRef.current.getBoundingClientRect();
            const num = toPercentage(handleLeft + (handleWidth / 2)) * 100;

            if (final) {
                onResizeFinish?.(num);
            } else {
                onResize?.(num);
            }
        }
    }, [boundingElementRef, onResize, onResizeFinish]);

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

    useEventHandler("pointerup", stopDragging, window, {
        passive: true,
    });

    useEventHandler("pointermove", (e) => {
        if (!(handleRef.current && dragging)) {
            return;
        }

        const { toPercentage, clampToBounds } = getBounds(Direction.VERTICAL, boundingElementRef.current);
        const percent = toPercentage(clampToBounds(e.clientX));

        handleRef.current.style.setProperty("--drag-offset", `${percent}`);
        dispatchResize();
    }, window, {
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
                ["--initial-drag-offset" as any]: initialPosition,
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
                }
                props.onPointerDown?.(e);
            }}
        />
    );
}

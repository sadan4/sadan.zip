import { useEventHandler } from "@/hooks/eventListener";
import cn from "@/utils/cn";

import type { ResizeHandleProps } from ".";
import { Direction, getBounds } from "./bounds";
import styles from "./styles.module.scss";

import invariant from "invariant";
import { useCallback, useEffect, useRef, useState } from "react";

export interface HorizontalResizeHandleProps extends ResizeHandleProps {
}

export function Horizontal({
    className,
    style,
    initialPosition = 0.5,
    boundingElementRef,
    onResizeFinish,
    onResize,
    onReset,
    onDoubleClick,
    ...props
}: ResizeHandleProps) {
    invariant(initialPosition >= 0 && initialPosition <= 1, "Invalid initial position");

    const handleRef = useRef<HTMLDivElement>(null);
    const [dragging, setDragging] = useState(false);

    const dispatchResize = useCallback((final = false) => {
        if (handleRef.current && boundingElementRef.current) {
            const { toPercentage } = getBounds(Direction.HORIZONTAL, boundingElementRef.current);
            const { top: handleTop, height: handleHeight } = handleRef.current.getBoundingClientRect();
            const num = toPercentage(handleTop + (handleHeight / 2)) * 100;

            if (final) {
                onResizeFinish?.(num);
            } else {
                onResize?.(num);
            }
        }
    }, [boundingElementRef, onResize, onResizeFinish]);

    const stopDragging = useCallback(() => {
        setDragging(false);
        dispatchResize(true);
    }, [dispatchResize]);

    useEventHandler("pointermove", (e) => {
        if (!(handleRef.current && dragging)) {
            return;
        }

        const { toPercentage, clampToBounds } = getBounds(Direction.HORIZONTAL, boundingElementRef.current);
        const percent = toPercentage(clampToBounds(e.clientY));

        handleRef.current.style.setProperty("--drag-offset", `${percent}`);
        dispatchResize();
    }, window, {
        passive: true,
    });

    useEventHandler("pointerup", stopDragging, window, {
        passive: true,
    });

    const dispatchReset = useCallback(() => {
        onReset?.();
    }, [onReset]);

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

    const reset = useCallback(() => {
        handleRef.current?.style.removeProperty("--drag-offset");
        stopDragging();
        dispatchResize();
        dispatchReset();
    }, [dispatchReset, dispatchResize, stopDragging]);

    useEffect(() => {
        dispatchResize(true);
        dispatchResize(false);
    }, [dispatchResize]);

    return (
        <div
            ref={handleRef}
            className={cn(styles.horizontalHandle, dragging && styles.dragging, className)}
            style={{
                ["--initial-drag-offset" as any]: initialPosition,
                ...style,
            }}
            {...props}
            onDoubleClick={(e) => {
                reset();
                onDoubleClick?.(e);
            }}
            onPointerDown={(e) => {
                if (e.isPrimary) {
                    setDragging(true);
                }
                props.onPointerDown?.(e);
            }}
        />
    );
}

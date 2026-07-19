import { type SpringRef, useSpring } from "@react-spring/web";

import { type BaseBorderHoldProps, borderHoldAnimConfig } from "./common";
import { BorderProgress } from "../BorderProgress";

import { type Ref, useCallback, useImperativeHandle, useRef } from "react";

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
    onPointerDown?(): void;
    /**
     * higher number, thinner border.
     * 
     * min value: 1
     *
     * @default 10
     */
    widthCoefficient?: number;
}

export function BorderHoldRounded({
    children,
    onHold,
    onPointerDown,
    ref,
    widthCoefficient = 10,
}: BorderHoldRoundedProps) {
    const borderRef = useRef<BorderProgress.Handle | null>(null);
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
                        const value = progress.value.progress;

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
                        if (!progress.cancelled && progress.value.progress <= 5) {
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
        recalculateBorder() {
            borderRef.current?.recalculateBorder();
        },
        reactSpringApi: api,
        onStopHold,
    }), [api, onStopHold]);


    return (
        <BorderProgress
            ref={borderRef}
            progress={progress}
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
            onPointerUp={() => {
                onStopHold();
            }}
            onPointerLeave={() => {
                onStopHold();
            }}
            pathStyle={{
                opacity,
            }}
            widthCoefficient={widthCoefficient}
        >
            {children}
        </BorderProgress>
    );
}

import { type SpringRef, useSpring } from "@react-spring/web";

import { borderHoldAnimConfig } from "./util";
import { BorderProgress } from "../BorderProgress";

import { type PropsWithChildren, type Ref, useCallback, useImperativeHandle, useRef } from "react";

export interface BorderHoldSpring {
    progress: number;
    opacity: number;
}

export declare namespace BorderHold {
    export interface Handle {
        recalculateBorder(): void;
        reactSpringApi: SpringRef<BorderHoldSpring>;
        onStopHold(): void;
    }
}

declare module "react" {
    interface CSSProperties {
        "--border-hold-progress"?: number;
    }
}

export interface BorderHoldProps extends PropsWithChildren {
    onHold?(): void;
    ref?: Ref<BorderHold.Handle | null>;
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

export function BorderHold({
    children,
    onHold,
    onPointerDown,
    ref,
    widthCoefficient = 10,
}: BorderHoldProps) {
    const borderRef = useRef<BorderProgress.Handle | null>(null);
    const dispatchedRef = useRef(false);
    const stoppingRef = useRef(false);

    const [{ progress, opacity }, api] = useSpring(() => ({
        from: {
            progress: 0,
            opacity: 0,
        },
    }));

    const onStartHold = useCallback(() => {
        stoppingRef.current = false;
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
        stoppingRef.current = true;
        api.start({
            async to(next) {
                await next({
                    progress: 0,
                    onChange(progress) {
                        if (!progress.cancelled && progress.value.progress <= 5) {
                            // react spring doesn't like this, but it works
                            next({
                                opacity: 0,
                            }).catch(() => { });
                            stoppingRef.current = false;
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
                console.log("Pointer down");
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
                // FIXME: hacky workaround for things breaking on touchscreens
                // which fire an onPointerUp event AND an onPointerLeave event
                if (stoppingRef.current)
                    return;
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

import { useToaster } from "@/hooks/toaster";
import { type Toast as IToast, ToastPosition, ToastType } from "@/stores/ToastStore";
import { cn } from "@/utils/cn";
import { animated, useSpringValue, useTransition } from "@react-spring/web";

import styles from "./styles.module.scss";
import { BorderProgress } from "../effects/BorderProgress";

import { CircleXIcon, InfoIcon, TriangleAlertIcon } from "lucide-react";
import { type ReactNode, useCallback, useEffect, useLayoutEffect } from "react";
import { useStore } from "zustand";

const typeStyle = {
    [ToastType.UNKNOWN]: styles.unknown,
    [ToastType.INFO]: styles.info,
    [ToastType.SUCCESS]: styles.success,
    [ToastType.WARNING]: styles.warning,
    [ToastType.ERROR]: styles.error,
} satisfies Record<IToast["type"], string>;

const toastIcon = {
    [ToastType.INFO]: <InfoIcon />,
    [ToastType.UNKNOWN]: null,
    [ToastType.SUCCESS]: null,
    [ToastType.WARNING]: <TriangleAlertIcon />,
    [ToastType.ERROR]: <CircleXIcon />,
} satisfies Record<IToast["type"], ReactNode>;

function useToastQueue(): [currentToast: IToast | null, next: () => void] {
    const store = useToaster();
    const currentToast = useStore(store, ({ _toasts: [t] }) => t) ?? null;

    const nextToast = useCallback(() => {
        store.getState().popToast();
    }, [store]);

    return [currentToast, nextToast];
}

const posClassMap = {
    [ToastPosition.TOP]: styles.posTop,
    [ToastPosition.BOTTOM]: styles.posBottom,
} satisfies Record<IToast["pos"], string>;

interface ToastTransitionProps {
    opacity: number;
    top?: number;
    bottom?: number;
}

type ToastTransitionConfig = Parameters<typeof useTransition<IToast | null, ToastTransitionProps>>[1];

const TOAST_TRANSITION_PROPS = Object.freeze({
    from: {
        opacity: 0,
        top: 0,
        bottom: 0,
    },
    enter: {
        opacity: 1,
        top: 30,
        bottom: 30,
    },
    leave: {
        opacity: 0,
        top: 0,
        bottom: 0,
    },
} satisfies ToastTransitionConfig);

function pickTransitionProps<T extends Record<keyof ToastTransitionProps, any>>(pos: ToastPosition, props: T) {
    switch (pos) {
        case ToastPosition.TOP: {
            const { bottom: _, ...rest } = props;

            return rest;
        }
        case ToastPosition.BOTTOM: {
            const { top: _, ...rest } = props;

            return rest;
        }
    }
}

export function Toaster() {
    const [cur, next] = useToastQueue();
    const transitions = useTransition(cur, TOAST_TRANSITION_PROPS);

    return (
        <div className={styles.toasterContainer}>
            {transitions((style, toast) => {
                if (!toast) {
                    return;
                }

                const props = pickTransitionProps(toast.pos, style);

                return (
                    <animated.div
                        className={cn(styles.toastWrapper, posClassMap[toast.pos], typeStyle[toast.type])}
                        style={props}
                    >
                        <Toast
                            toast={toast}
                            onDone={() => {
                                next();
                            }}
                        />
                    </animated.div>
                );
            })}
        </div>
    );
}


interface ToastProps {
    toast: IToast;
    onDone: () => void;
}

function Toast({ toast: { duration, render, type }, onDone }: ToastProps) {
    const progress = useSpringValue(0, {
        config: {
            duration,
        },
        onRest({ cancelled }) {
            if (!cancelled) {
                onDone();
            }
        },
    });

    useEffect(() => {
        progress.start(100);
    }, [progress]);

    // If this toast is popped before the animation finishes, cancel it so we don't call onDone twice
    // It needs to be a layout effect so that the dtor fires before onRest is called because we are being unmounted
    useLayoutEffect(() => {
        return () => {
            progress.stop(true);
        };
    }, [progress]);

    return (
        <BorderProgress
            progress={progress}
            onMouseOver={() => {
                progress.pause();
            }}
            onMouseOut={() => {
                progress.resume();
            }}
            onContextMenu={(e) => {
                e.preventDefault();
                // when we pop ourselves, we will get unmounted before the animation finishes
                // so we don't have to worry about popping twice
                onDone();
            }}
        >
            <div className={cn(styles.toast, typeStyle[type])}>
                {toastIcon[type]}
                <div>
                    {render()}
                </div>
            </div>
        </BorderProgress>
    );
}

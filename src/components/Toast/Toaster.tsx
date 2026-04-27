import { useToaster } from "@/hooks/toaster";
import { type Toast, ToastPosition } from "@/stores/ToastStore";
import { cn } from "@/utils/cn";
import { animated, useTransition } from "@react-spring/web";

import styles from "./styles.module.scss";

import { type RefObject, useCallback, useEffect } from "react";
import { useStore } from "zustand";

export interface ToasterProps {
    containerRef: RefObject<HTMLDivElement | null>;
}

function useToastQueue(): [currentToast: Toast | null, next: () => void] {
    const store = useToaster();
    const currentToast = useStore(store, ({ _toasts: [t] }) => t) ?? null;

    const nextToast = useCallback(() => {
        const { _toasts: [, ..._toasts] } = store.getState();

        store.setState({ _toasts });
    }, [store]);

    return [currentToast, nextToast];
}

const posClassMap = {
    [ToastPosition.TOP]: styles.posTop,
} satisfies Record<Toast["pos"], string>;

export function Toaster({ containerRef }: ToasterProps) {
    const [cur, next] = useToastQueue();

    containerRef;
    cur;
    next;

    const transitions = useTransition(cur, {
        from: {
            opacity: 0,
            top: 0,
        },
        enter: {
            opacity: 1,
            top: 30,
        },
        leave: {
            opacity: 0,
            top: 0,
        },
    });

    useEffect(() => {
        if (!cur) {
            return;
        }

        const id = setTimeout(() => {
            next();
        }, cur.duration);

        return () => clearTimeout(id);
    }, [next, cur]);


    return (
        <div className={styles.toasterContainer}>
            {transitions((style, toast) => {
                console.log("rendering toast with style", style);
                return toast && (
                    <animated.div
                        className={cn(styles.toastWrapper, posClassMap[toast.pos])}
                        style={style}
                    >
                        TODO: render toast
                    </animated.div>
                );
            })}
        </div>
    );
}

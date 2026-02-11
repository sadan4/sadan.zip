import { useIsomorphicLayoutEffect } from "./isomorphicLayoutEffect";
import { useRecent } from "./recent";

import type { RefObject } from "react";

export interface UseResizeObserverCallback {
    (entry: ResizeObserverEntry, observer: ResizeObserver): void;
}

// FIXME: fix spelling
export function useReiszeObserverFromRef<T extends Element>(
    target: RefObject<T | null>,
    callback: UseResizeObserverCallback,
) {
    return useResizeObserver(target.current, callback);
}
export function useResizeObserver<T extends Element>(
    target: RefObject<T> | T | null,
    callback: UseResizeObserverCallback,
) {
    const cb = useRecent(callback);

    useIsomorphicLayoutEffect(() => {
        if (!target)
            return;

        const el = (() => {
            if (target instanceof Element) {
                return target;
            }
            return target.current;
        })();

        const observer = new ResizeObserver((entries, observer) => {
            const seen = new Set<Element>();

            for (const entry of entries) {
                if (seen.has(entry.target))
                    continue;
                seen.add(entry.target);
                cb.current(entry, observer);
            }
        });

        observer.observe(el);

        return () => {
            observer.disconnect();
        };
    }, [cb, target]);
}

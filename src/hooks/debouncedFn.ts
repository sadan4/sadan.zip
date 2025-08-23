import { useRecent } from "./recent";

import { useRef } from "react";

export function useDebouncedFn<T extends (...args: any[]) => void>(fn: T, delay = 300): T {
    const fnRef = useRecent(fn);
    const timeoutRef = useRef<NodeJS.Timeout>(0 as any);

    if (delay === 0) {
        return fn;
    }
    return ((...args: Parameters<T>) => {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => {
            fnRef.current(...args);
        }, delay);
    }) as T;
}

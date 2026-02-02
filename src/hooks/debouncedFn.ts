import { useRecent } from "./recent";

import { useCallback, useEffect, useRef } from "react";

export function useDebouncedFn<T extends (...args: any[]) => void>(fn: T, delay = 300, removeOnUnmount = false): T {
    const delayRef = useRecent(delay);
    const removeOnUnmountRef = useRecent(removeOnUnmount);
    const fnRef = useRecent(fn);
    const timeoutRef = useRef<NodeJS.Timeout>(0 as any);

    useEffect(() => {
        return () => {
            // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional
            if (removeOnUnmountRef.current) {
                clearTimeout(timeoutRef.current);
            }
        };
    }, [removeOnUnmountRef]);

    return useCallback((...args: Parameters<T>): void => {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => {
            fnRef.current(...args);
        }, delayRef.current);
    }, [delayRef, fnRef]) as T;
}

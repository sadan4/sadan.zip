import { useRecent } from "./recent";

import { useCallback, useRef } from "react";

const SYM_UNCALLED = Symbol("uncalled");

export function useOnce<T extends (...args: any[]) => any>(fn: T): (...args: Parameters<T>) => ReturnType<T> {
    const result = useRef<ReturnType<T> | typeof SYM_UNCALLED>(SYM_UNCALLED);
    const cb = useRecent(fn);

    return useCallback((...args: Parameters<T>): ReturnType<T> => {
        if (result.current === SYM_UNCALLED) {
            result.current = cb.current(...args);
        }
        return result.current as ReturnType<T>;
    }, [cb]);
}

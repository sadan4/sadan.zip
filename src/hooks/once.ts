import { useRecent } from "./recent";

import { useCallback, useRef } from "react";

const SYM_UNCALLED = Symbol("uncalled");

export function useOnce<T extends (...args: any[]) => any>(fn: T): (...args: Parameters<T>) => ReturnType<T> {
    const resultRef = useRef<ReturnType<T> | typeof SYM_UNCALLED>(SYM_UNCALLED);
    const cb = useRecent(fn);

    return useCallback((...args: Parameters<T>): ReturnType<T> => {
        if (resultRef.current === SYM_UNCALLED) {
            resultRef.current = cb.current(...args);
        }
        return resultRef.current as ReturnType<T>;
    }, [cb]);
}

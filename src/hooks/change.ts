import { useRecent } from "./recent";

import { useEffect, useRef } from "react";

const SYM_INITIAL = Symbol("useChange.INITIAL");

/**
 * compares with ===
 *
 * does **not** fire on first render
 */
export function useChange<T>(cb: (prev: T, cur: T) => void, value: T): void {
    const prevRef = useRef<T | typeof SYM_INITIAL>(SYM_INITIAL);
    const cbRef = useRecent(cb);

    useEffect(() => {
        if (prevRef.current === SYM_INITIAL) {
            prevRef.current = value;
            return;
        } if (prevRef.current === value) {
            return;
        }
        cbRef.current(prevRef.current, value);
        prevRef.current = value;
    }, [value, cbRef]);
}

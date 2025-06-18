import deepEqual from "fast-deep-equal";
import { useRef } from "react";

export function useDeepMemo<T>(value: T): T {
    const ref = useRef<T>(value);

    if (!deepEqual(value, ref.current)) {
        ref.current = value;
    }

    return ref.current;
}

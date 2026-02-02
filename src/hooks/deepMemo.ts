import { deepEqual } from "@/utils/obj";

import { useRef } from "react";

/* eslint-disable react-hooks/refs -- valid use case */
export function useDeepMemo<T>(value: T): T {
    const ref = useRef<T>(value);

    if (!deepEqual(value, ref.current)) {
        ref.current = value;
    }

    return ref.current;
}
/* eslint-enable */

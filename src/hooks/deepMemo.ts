import { deepEqual } from "@/utils/obj";

import { useRef } from "react";

export function useDeepMemo<T>(value: T): T {
    const ref = useRef<T>(value);

    if (!deepEqual(value, ref.current)) {
        // eslint-disable-next-line react-hooks/refs
        ref.current = value;
    }

    return ref.current;
}

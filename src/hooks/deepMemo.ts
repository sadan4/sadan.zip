import { deepEqual } from "@/utils/obj";

import { useRef } from "react";

export function useDeepMemo<T>(value: T): T {
    const ref = useRef<T>(value);

    // eslint-disable-next-line react/react-compiler
    if (!deepEqual(value, ref.current)) {
        // eslint-disable-next-line react/react-compiler
        ref.current = value;
    }

    // eslint-disable-next-line react/react-compiler
    return ref.current;
}

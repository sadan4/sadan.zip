// eslint-disable react/react-compiler
import { shallowEqual } from "@/utils/obj";

import { useRef } from "react";

export function useShallowMemo<T>(value: T): T {
    const ref = useRef<T>(value);

    if (!shallowEqual(value, ref.current)) {
        ref.current = value;
    }

    return ref.current;
}

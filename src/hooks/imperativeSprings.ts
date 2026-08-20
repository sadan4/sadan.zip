import { mapObject } from "@/utils/obj";
import { type SpringConfig, type SpringValue, useSpringValue } from "@react-spring/web";

import { useRef } from "react";

type mapSpringValue<T extends Record<string, number>> = {
    [K in keyof T]: SpringValue<T[K]>;
};

export function useImperativeSprings<T extends Record<string, number>>(
    initialValue: T,
    config: SpringConfig = {},
): mapSpringValue<T> {
    const initRef = useRef(initialValue);

    // oxlint-disable-next-line react/refs
    return mapObject(initRef.current, (initialValue) => {
        // oxlint-disable-next-line react-x/rules-of-hooks
        return useSpringValue(initialValue, {
            config,
        });
    }) as any as mapSpringValue<T>;
}

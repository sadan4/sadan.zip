import { mapObject } from "@/utils/functional";
import { type SpringConfig, type SpringValue, useSpringValue } from "@react-spring/web";

import { useRef } from "react";

type mapSpringValue<T extends Record<string, number>> = {
    [K in keyof T]: SpringValue<T[K]>;
};

export function useImperativeSprings<T extends Record<string, number>>(
    initialValue: T,
    config: SpringConfig = {},
): mapSpringValue<T> {
    const init = useRef(initialValue);

    return mapObject(init.current, (initialValue) => {
        // eslint-disable-next-line react-hooks/react-compiler
        // eslint-disable-next-line react-hooks/rules-of-hooks
        return useSpringValue(initialValue, {
            config,
        });
    }) as any as mapSpringValue<T>;
}

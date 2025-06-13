import useResizeObserver from "@react-hook/resize-observer";

import { useLayoutEffect, useState } from "react";

export function useSize<T extends Element>(target: T | null): DOMRect | undefined {
    const [size, setSize] = useState<any>();

    useLayoutEffect(() => {
        target && setSize(target.getBoundingClientRect());
    }, [target]);

    useResizeObserver(target, (entry) => {
        setSize(entry.target.getBoundingClientRect());
    });

    return size;
}

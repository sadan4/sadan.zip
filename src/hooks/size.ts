import useResizeObserver from "@react-hook/resize-observer";

import { useEventHandler } from "./eventListener";
import { useForceUpdater } from "./forceUpdater";

import { useLayoutEffect, useState } from "react";

export function useSize<T extends Element>(target: T | null): DOMRect | undefined {
    const [size, setSize] = useState<any>();
    const [, forceUpdate] = useForceUpdater();

    useLayoutEffect(() => {
        target && setSize(target.getBoundingClientRect());
    }, [target]);

    useResizeObserver(target, (entry) => {
        setSize(entry.target.getBoundingClientRect());
    });

    // window resizing could also change position but not size
    useEventHandler("resize", () => {
        // dom rects are mutable, so we can't compare them to see if they changed
        // window will not be resized *that* often
        setSize(target?.getBoundingClientRect());
        forceUpdate();
    });

    return size;
}

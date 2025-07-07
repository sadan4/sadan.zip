import useResizeObserver from "@react-hook/resize-observer";

import { useEventHandler } from "./eventListener";
import { useForceUpdater } from "./forceUpdater";
import { useRecent } from "./recent";

import { useLayoutEffect, useState } from "react";

export function useSize<T extends Element>(target: () => (T | null)): DOMRect | undefined {
    const [size, setSize] = useState<any>();
    const [, forceUpdate] = useForceUpdater();
    const targetRef = useRecent(target);

    useLayoutEffect(() => {
        targetRef.current() && setSize(targetRef.current()!
            .getBoundingClientRect());
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [targetRef, target()]);

    useResizeObserver(target(), (entry) => {
        setSize(entry.target.getBoundingClientRect());
    });

    // window resizing could also change position but not size
    useEventHandler("resize", () => {
        // dom rects are mutable, so we can't compare them to see if they changed
        // window will not be resized *that* often
        setSize(targetRef.current()
            ?.getBoundingClientRect());
        forceUpdate();
    });

    return size;
}

import { measureRect } from "@/utils/dom";
import useResizeObserver from "@react-hook/resize-observer";

import { useEventHandler } from "./eventListener";

import { useLayoutEffect, useState } from "react";

export function useRect(el: Element | null): DOMRect | undefined {
    const [size, setSize] = useState<any>();

    useLayoutEffect(() => {
        if (el) {
            setSize(measureRect(el));
        }
    }, [el]);

    useResizeObserver(el, (entry) => {
        setSize(measureRect(entry.target));
    });

    // window resizing could also change position but not size
    useEventHandler("resize", () => {
        // dom rects are mutable, so we can't compare them to see if they changed
        // window will not be resized *that* often
        if (el) {
            setSize(measureRect(el));
        }
    });

    useEventHandler("scroll", () => {
        if (el) {
            setSize(measureRect(el));
        }
    });

    return size;
}

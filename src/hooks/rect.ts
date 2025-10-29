import { ScrollAreaContext } from "@/components/layout/ScrollArea/context";
import { measureRect } from "@/utils/dom";
import { deepEqual } from "@/utils/obj";
import useResizeObserver from "@react-hook/resize-observer";

import { useEventHandler } from "./eventListener";

import { use, useEffect, useRef, useState } from "react";

export function useRect(el: Element | null, extraDeps: unknown[] = []): DOMRect | undefined {
    const [size, _setSize] = useState<DOMRect>();
    const sizeRef = useRef(size);
    const { ref: { current: scroller } } = use(ScrollAreaContext);


    function setSize(newSize: DOMRect) {
        sizeRef.current = newSize;
        _setSize(newSize);
    }

    useEffect(() => {
        if (el) {
            const newRect = measureRect(el).toJSON();

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    }, [el, ...extraDeps]);

    useEventHandler("scrollend", () => {
        if (el) {
            const newRect = measureRect(el).toJSON();

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    }, scroller);

    useResizeObserver(el, () => {
        if (el) {
            const newRect = measureRect(el).toJSON();

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    // window resizing could also change position but not size
    useEventHandler("resize", () => {
        // dom rects are mutable, so we can't compare them to see if they changed
        // window will not be resized *that* often
        if (el) {
            const newRect = measureRect(el).toJSON();

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    useEventHandler("scroll", () => {
        if (el) {
            const newRect = measureRect(el).toJSON();

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    return size;
}

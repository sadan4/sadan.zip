import { ScrollAreaContext } from "@/components/layout/ScrollArea/context";
import { measureRect } from "@/utils/dom";
import { deepEqual, pick } from "@/utils/obj";
import useResizeObserver from "@react-hook/resize-observer";

import { useEventHandler } from "./eventListener";

import { type RefObject, use, useCallback, useEffect, useRef, useState } from "react";

function useRectMapper<T extends keyof DOMRect>(keys: T[]): (rect: DOMRect) => Pick<DOMRect, T> {
    return useCallback((rect) => {
        if (!keys.length) {
            return rect;
        }
        return pick(rect, keys);
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, keys);
}


export function useRect(el: Element | null, extraDeps?: unknown[]): DOMRect | undefined;
export function useRect<T extends keyof DOMRect>(
    el: Element | null,
    extraDeps: unknown[],
    keys: T[]
): Pick<DOMRect, T> | undefined;
export function useRect(
    el: Element | null,
    extraDeps: unknown[] = [],
    keys: (keyof DOMRect)[] = [],
): DOMRect | undefined {
    const mapper = useRectMapper(keys);
    const [size, _setSize] = useState<DOMRect>();
    const sizeRef = useRef(size);
    const { ref: { current: scroller } } = use(ScrollAreaContext);


    function setSize(newSize: DOMRect) {
        sizeRef.current = newSize;
        _setSize(newSize);
    }

    useEffect(() => {
        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [el, mapper, ...extraDeps]);

    useEventHandler("scrollend", () => {
        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    }, scroller);

    useResizeObserver(el, () => {
        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

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
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    useEventHandler("scroll", () => {
        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    return size;
}


export function useRectFromRef(ref: RefObject<Element | null>, extraDeps?: unknown[]): DOMRect | undefined;
export function useRectFromRef<T extends keyof DOMRect>(
    ref: RefObject<Element | null>,
    extraDeps: unknown[],
    keys: T[]
): Pick<DOMRect, T> | undefined;
export function useRectFromRef(
    ref: RefObject<Element | null>,
    extraDeps: unknown[] = [],
    keys: (keyof DOMRect)[] = [],
): DOMRect | undefined {
    const mapper = useRectMapper(keys);
    const [size, _setSize] = useState<DOMRect>();
    const sizeRef = useRef(size);
    const { ref: { current: scroller } } = use(ScrollAreaContext);


    function setSize(newSize: DOMRect) {
        sizeRef.current = newSize;
        _setSize(newSize);
    }

    useEffect(() => {
        const el = ref.current;

        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [ref, mapper, ...extraDeps]);

    useEventHandler("scrollend", () => {
        const el = ref.current;

        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    }, scroller);

    useResizeObserver(ref, () => {
        const el = ref.current;

        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    // window resizing could also change position but not size
    useEventHandler("resize", () => {
        const el = ref.current;
        // dom rects are mutable, so we can't compare them to see if they changed
        // window will not be resized *that* often

        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    useEventHandler("scroll", () => {
        const el = ref.current;

        if (el) {
            const newRect = mapper(measureRect(el).toJSON());

            if (deepEqual(sizeRef.current, newRect)) {
                return;
            }

            setSize(newRect);
        }
    });

    return size;
}

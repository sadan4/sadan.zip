import { useDeepMemo } from "./deepMemo";
import { useRecent } from "./recent";

import { useEffect } from "react";

type AllEventMaps = HTMLElementEventMap & DocumentEventMap & WindowEventMap;

export function useEventHandler<K extends keyof HTMLElementEventMap, E extends HTMLElement>(
    type: K,
    handler: (this: E, event: HTMLElementEventMap[K]) => void,
    element: E | null,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof DocumentEventMap>(
    type: K,
    handler: (this: Document, event: DocumentEventMap[K]) => void,
    element: Document | null,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof WindowEventMap>(
    type: K,
    handler: (this: Window, event: WindowEventMap[K]) => void,
    element?: Window,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof AllEventMaps, E extends HTMLElement | Document | Window | null>(
    type: K,
    handler: (this: E, ev: AllEventMaps[K]) => void,
    element?: E,
    _opts: AddEventListenerOptions = {},
) {
    const handlerRef = useRecent(handler);
    const opts = useDeepMemo(_opts);

    useEffect(() => {
        if (element === null)
            return;

        const el = element === undefined ? window : element;

        const wrappedHandler = function (this: typeof el, ev: AllEventMaps[K]) {
            return handlerRef.current.call(this as E, ev);
        };

        el.addEventListener(type, wrappedHandler as EventListenerOrEventListenerObject, opts);

        return () => {
            el.removeEventListener(type, wrappedHandler as EventListenerOrEventListenerObject);
        };
    }, [type, element, handlerRef, opts]);
}

import { useDeepMemo } from "./deepMemo";
import { useRecent } from "./recent";

import { useEffect } from "react";

type AllEventMaps = HTMLElementEventMap & DocumentEventMap & WindowEventMap;

export function useEventHandler<K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (event: HTMLElementEventMap[K]) => void,
    element: HTMLElement,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof DocumentEventMap>(
    type: K,
    handler: (event: DocumentEventMap[K]) => void,
    element: Document,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof WindowEventMap>(
    type: K,
    handler: (event: WindowEventMap[K]) => void,
    element?: Window,
    opts?: AddEventListenerOptions
): void;

export function useEventHandler<K extends keyof AllEventMaps>(
    type: K,
    handler: (event: AllEventMaps[K]) => void,
    element?: HTMLElement | Document | Window | null,
    _opts: AddEventListenerOptions = {},
) {
    const handlerRef = useRecent(handler);
    const opts = useDeepMemo(_opts);

    useEffect(() => {
        const el = element === undefined ? window : element;

        const wrappedHandler = (ev: AllEventMaps[K]) => {
            return handlerRef.current(ev);
        };

        el?.addEventListener(type, wrappedHandler as EventListenerOrEventListenerObject, opts);

        return () => {
            el?.removeEventListener(type, wrappedHandler as EventListenerOrEventListenerObject);
        };
    }, [type, element, handlerRef, opts]);
}

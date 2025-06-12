import { useRecent } from "./recent";

import { useEffect } from "react";

type AllEventMaps = HTMLElementEventMap & DocumentEventMap & WindowEventMap;

export function useEventHandler<K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (event: HTMLElementEventMap[K]) => void,
    element: HTMLElement
): void;

export function useEventHandler<K extends keyof DocumentEventMap>(
    type: K,
    handler: (event: DocumentEventMap[K]) => void,
    element: Document
): void;

export function useEventHandler<K extends keyof WindowEventMap>(
    type: K,
    handler: (event: WindowEventMap[K]) => void,
    element?: Window
): void;

export function useEventHandler<K extends keyof AllEventMaps>(
    type: K,
    handler: (event: AllEventMaps[K]) => void,
    element?: HTMLElement | Document | Window | null,
) {
    const handlerRef = useRecent(handler);

    useEffect(() => {
        const el = element === undefined ? window : element;

        const wrappedHandler = (ev: AllEventMaps[K]) => {
            return handlerRef.current(ev);
        };

        el?.addEventListener(type, wrappedHandler as EventListenerOrEventListenerObject);

        return () => {
            el?.removeEventListener(type, wrappedHandler as EventListenerOrEventListenerObject);
        };
    }, [type, element, handlerRef]);
}

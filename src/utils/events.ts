import { once } from "./functional";

type AllEventMaps = HTMLElementEventMap & DocumentEventMap & WindowEventMap;

export function disposableEventHandler<K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (event: HTMLElementEventMap[K]) => void,
    element: HTMLElement
): () => void;

export function disposableEventHandler<K extends keyof DocumentEventMap>(
    type: K,
    handler: (event: DocumentEventMap[K]) => void,
    element: Document
): () => void;

export function disposableEventHandler<K extends keyof WindowEventMap>(
    type: K,
    handler: (event: WindowEventMap[K]) => void,
    element?: Window
): () => void;

export function disposableEventHandler<K extends keyof AllEventMaps>(
    type: K,
    handler: (event: AllEventMaps[K]) => void,
    element?: HTMLElement | Document | Window | null,
): () => void {
    const el = element === undefined ? window : element;


    el?.addEventListener(type, handler as EventListenerOrEventListenerObject);

    return once(() => {
        el?.removeEventListener(type, handler as EventListenerOrEventListenerObject);
    });
}

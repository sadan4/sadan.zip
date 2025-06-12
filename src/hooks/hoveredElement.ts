import { useEventHandler } from "./eventListener";
import { useRecent } from "./recent";

import { useState } from "react";


export type ElementEqualityCallback = (oldElement: Element | null, newElement: Element | null) => boolean;

export function useHoveredElement(areElementsEqual?: ElementEqualityCallback): Element | null {
    const equalCallback = useRecent(areElementsEqual ?? (() => false));
    const [hoveredElement, setHoveredElement] = useState<Element | null>(null);

    useEventHandler("mouseover", (ev: MouseEvent) => {
        if (equalCallback.current(hoveredElement, ev.target as Element)) {
            return;
        }
        setHoveredElement(ev.target as Element);
    });
    return hoveredElement;
}

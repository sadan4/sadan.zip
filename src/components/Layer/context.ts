import { namedContext } from "@/utils/devtools";
import { assert } from "@/utils/error";
import { proxyLazy } from "@/utils/lazy";

export interface LayerContext {
    /**
     * starts at 0 being the layer closest to body
     */
    level: number;
    /**
     * null in the gap bewteen rendering the new layer and the ref being set
     */
    root: HTMLDivElement | null;
}


export const LayerContext = namedContext<LayerContext>(proxyLazy((): LayerContext => {
    const root = document.getElementById("root");

    assert(root instanceof HTMLDivElement, "Root element must be a div");

    return {
        level: 0,
        root,
    };
}), "LayerContext");

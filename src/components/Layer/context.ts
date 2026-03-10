import { createContext } from "react";


export type LayerElement = HTMLBodyElement | HTMLDivElement;
export interface LayerContext {
    /**
     * starts at 0 being the layer closest to body
     */
    level: number;
    /**
     * null in the gap bewteen rendering the new layer and the ref being set
     */
    root: LayerElement | null;
}


export const LayerContext = createContext<LayerContext>({
    level: 0,
    root: null,
});
LayerContext.displayName = "LayerContext";

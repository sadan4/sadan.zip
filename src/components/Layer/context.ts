import { namedContext } from "@/utils/devtools";

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


export const LayerContext = namedContext<LayerContext>({
    level: 0,
    root: null,
}, "LayerContext");

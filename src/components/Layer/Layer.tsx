import { LayerContext } from "./context";
import * as styles from "./styles.module.scss";

import { type PropsWithChildren, use, useMemo, useState } from "react";
import { createPortal } from "react-dom";

export interface LayerProps extends PropsWithChildren {

}

export function Layer({ children }: LayerProps) {
    const currentLayer = use(LayerContext);
    const [currentLayerRootRef, setCurrentLayerRootRef] = useState<HTMLDivElement | null>(null);

    const value = useMemo<LayerContext>(() => ({
        level: currentLayer.level + 1,
        root: currentLayerRootRef,
    }), [currentLayer.level, currentLayerRootRef]);

    return (
        <LayerContext value={value}>
            <div
                ref={setCurrentLayerRootRef}
                data-layer-level={currentLayer.level + 1}
                className={styles.layer}
            >
                {children}
            </div>
        </LayerContext>
    );
}

export interface LayerPortalProps extends PropsWithChildren {
}

export function LayerPortal({ children }: LayerPortalProps) {
    const ctx = use(LayerContext);

    return (
        <>
            {ctx.root && createPortal(
                children,
                ctx.root,
            )}
        </>
    );
}

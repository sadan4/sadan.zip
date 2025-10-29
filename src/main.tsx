import { installF8Break, uninstallF8Break } from "@/utils/devtools";
import { createRouter, RouterProvider } from "@tanstack/react-router";

import { LayerContext } from "./components/Layer/context";
import { assert } from "./utils/error";
import { routeTree } from "./routeTree.gen";

import { useEffect, useState } from "react";

const router = createRouter({
    routeTree,
    scrollRestoration: true,
    defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
    interface Register {
        router: typeof router;
    }
}


export interface RouterContext {
    solidBg?: boolean;
}

export function App() {
    const [ctx, setCtx] = useState<LayerContext>({
        level: 0,
        root: null,
    });

    useEffect(() => {
        installF8Break();

        const root = document.getElementById("root");

        assert(root instanceof HTMLDivElement);

        setCtx({
            level: 0,
            root,
        });

        return uninstallF8Break;
    }, []);

    return (
        <LayerContext value={ctx}>
            <RouterProvider router={router} />
        </LayerContext>
    );
}

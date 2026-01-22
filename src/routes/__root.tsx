import { LayerContext } from "@/components/Layer/context";
import { installF8Break, uninstallF8Break } from "@/utils/devtools";
import { assert } from "@/utils/error";
import { dedent } from "@/utils/string";
import { createRootRouteWithContext, HeadContent, Outlet, Scripts } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

import rootCssUrl from "../index.css?url";

import { useEffect, useState } from "react";

export const Route = createRootRouteWithContext<RouterContext>()({
    component: RootComponent,
    head() {
        return {
            links: [
                // ...import.meta.env.PROD
                //     ? [
                {
                    rel: "stylesheet",
                    href: rootCssUrl,
                },
                // ] satisfies AnyRouteMatch["links"]
                // : [],
            ],
            meta: [
                {
                    charSet: "UTF-8",
                },
                {
                    title: "sadan",
                },
                {
                    name: "viewport",
                    content: "width=device-width, initial-scale=1.0",
                },
                {
                    name: "referrer",
                    content: "same-origin",
                },
            ],
            scripts: [
                ...!import.meta.env.PROD
                    ? [
                        {
                            type: "module",
                            children: dedent/* js */`
                                import injectIntoGlobalHook from "/@react-refresh"
                                injectIntoGlobalHook(window)
                                window.$RefreshReg$ = () => {}
                                window.$RefreshSig$ = () => (type) => type
                                window.__vite_plugin_react_preamble_installed__ = true
                            `,
                        },
                        {
                            type: "module",
                            src: "/@vite/client",
                        },
                    ]
                    : [],
                {
                    type: "module",
                    src: import.meta.env.PROD ? "/client.js" : "/src/client.tsx",
                },
            ],
        };
    },
});

export interface RouterContext {
    /**
     * unused rn, remove optional if ever used if needed
     */
    head?: string;
}

function RootComponent() {
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
        <html
            lang="en"
            // its common for extensions to add styles and classes to the html element, which generates erroneous hydration errors
            suppressHydrationWarning
        >
            <head>
                <HeadContent />
            </head>
            <body>
                <div
                    id="root"
                    className="h-screen w-screen"
                >
                    <LayerContext value={ctx}>
                        <Outlet />
                        <TanStackRouterDevtools position="bottom-right" />
                    </LayerContext>
                </div>
                <Scripts />
            </body>
        </html>
    );
}

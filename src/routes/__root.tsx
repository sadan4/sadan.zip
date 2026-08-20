import { LayerContext } from "@/components/Layer/context";
import { ToastContainer } from "@/components/Toast";
import { NotFoundPage } from "@/routes/-404";
import { installF8Break, uninstallF8Break } from "@/utils/devtools";
import { assert } from "@/utils/error";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type AnyRouteMatch, createRootRoute, HeadContent, Scripts } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

import rootCssFontUrl from "../assets/ComicShannsMono-Regular.woff2?url";
import rootCss from "../index.css?url";

import { useEffect, useState } from "react";

export const Route = createRootRoute({
    loader({ location: { publicHref } }) {
        return {
            publicHref,
            notFoundQuoteSeed: Math.floor(Math.random() * 1_000_000_000),
        };
    },
    head(ctx) {
        const { pageTitle, description, imageUrl } = ctx.matches.at(-1)!.staticData;

        const meta: AnyRouteMatch["meta"] = [
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
            {
                property: "og:title",
                content: pageTitle ? `sadan - ${pageTitle}` : "sadan",
            },
            {
                property: "og:description",
                content: description ?? "My silly website",
            },
            {
                property: "og:image",
                content: imageUrl ? `https://sadan.zip${imageUrl}` : "https://sadan.zip/assets/avatar.webp",
            },
        ];

        const { publicHref } = ctx.loaderData ?? {};

        if (publicHref) {
            meta.push({
                property: "og:url",
                content: new URL(publicHref, "https://sadan.zip").toString(),
            });
        }

        return {
            meta,
            links: [
                // preload the font nested in the root css
                {
                    rel: "preload",
                    as: "font",
                    href: rootCssFontUrl,
                },
                {
                    rel: "stylesheet",
                    href: rootCss,
                },
            ],
        };
    },

    notFoundComponent: NotFoundPage,
    shellComponent: RootComponent,
});

const queryClient = new QueryClient();

declare global {
    interface Window {
        __TANSTACK_QUERY_CLIENT__: QueryClient;
    }
}

if (!import.meta.env.SSR) {
    window.__TANSTACK_QUERY_CLIENT__ = queryClient;
}

function RootComponent({ children }: { children: React.ReactNode; }) {
    const [layerCtx, setLayerCtx] = useState<LayerContext>({
        level: 0,
        root: null,
    });

    useEffect(() => {
        if (import.meta.env.DEV) {
            if (Error.stackTraceLimit < 15) {
                Error.stackTraceLimit = 15;
            }
        }
        installF8Break();

        const root = document.getElementById("root");

        assert(root instanceof HTMLBodyElement, "Failed to find root element");

        // this is only ever called once
        // oxlint-disable-next-line react/react-compiler
        setLayerCtx({
            level: 0,
            root,
        });

        return uninstallF8Break;
    }, []);

    return (
        <html
            lang="en"
            suppressHydrationWarning
        >
            <head>
                <HeadContent />
            </head>
            <body id="root">
                <QueryClientProvider client={queryClient}>
                    <LayerContext value={layerCtx}>
                        <ToastContainer>
                            {children}
                        </ToastContainer>
                    </LayerContext>
                    <TanStackDevtools
                        config={{
                            position: "bottom-right",
                        }}
                        plugins={[
                            {
                                name: "Tanstack Router",
                                render: <TanStackRouterDevtoolsPanel />,
                            },
                        ]}
                    />
                </QueryClientProvider>
                <Scripts />
            </body>
        </html>
    );
}

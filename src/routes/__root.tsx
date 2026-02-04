import { LayerContext } from "@/components/Layer/context";
import { installF8Break, uninstallF8Break } from "@/utils/devtools";
import { assert } from "@/utils/error";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { type AnyRouteMatch, createRootRoute, HeadContent, Scripts } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

import rootCss from "../index.css?url";

import { useEffect, useState } from "react";

export const Route = createRootRoute({
    loader({ location: { publicHref } }) {
        return {
            publicHref,
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
                {
                    rel: "stylesheet",
                    href: rootCss,
                },
            ],
        };
    },

    shellComponent: RootComponent,
});

function RootComponent({ children }: { children: React.ReactNode; }) {
    const [layerCtx, setCtx] = useState<LayerContext>({
        level: 0,
        root: null,
    });

    useEffect(() => {
        installF8Break();

        const root = document.getElementById("root");

        assert(root instanceof HTMLBodyElement, "Failed to find root element");

        setCtx({
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
                <LayerContext value={layerCtx}>
                    {children}
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
                <Scripts />
            </body>
        </html>
    );
}

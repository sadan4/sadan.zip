import { installF8Break } from "@/utils/devtools";

import "./app.scss";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider, useLoaderData as useLoaderData_ } from "react-router";

installF8Break();

export interface LoaderData {
    config: {
        solidBg?: boolean;
        noCursor?: boolean;
    };
}

export function useLoaderData() {
    return useLoaderData_<LoaderData>();
}

const router = createBrowserRouter([
    {
        path: "/",
        HydrateFallback: () => null,
        children: [
            {
                index: true,
                async lazy() {
                    const Component = (await import("./pages")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "e",
                loader(): LoaderData {
                    return {
                        config: {
                            solidBg: true,
                            noCursor: true,
                        },
                    };
                },
                async lazy() {
                    const Component = (await import("./pages/e")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "storybook",
                loader() {
                    location.pathname = "/storybook/index.html";
                },
            },
            {
                path: "demangler",
                async lazy() {
                    const Component = (await import("./pages/demangler")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "minky",
                async lazy() {
                    const Component = (await import("./pages/minky")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "components",
                async lazy() {
                    const Component = (await import("./pages/components")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "discord-intl",
                async lazy() {
                    const Component = (await import("./pages/discord-intl")).default;

                    return {
                        Component,
                    };
                },
            },
        ],
    },
]);

createRoot(document.body)
    .render((
        <StrictMode>
            <RouterProvider router={router} />
        </StrictMode>
    ));

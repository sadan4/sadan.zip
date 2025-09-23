import { installF8Break } from "@/utils/devtools";

import "./app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router";

installF8Break();

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

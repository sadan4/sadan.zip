import { installF8Break } from "@/utils/devtools";

import "./index.css";
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
        ],
    },
]);

createRoot(document.body)
    .render((
        <StrictMode>
            <RouterProvider router={router} />
        </StrictMode>
    ));

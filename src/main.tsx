import { installF8Break, namedContext, uninstallF8Break } from "@/utils/devtools";

import "./app.scss";
import { StrictMode, useContext, useEffect } from "react";
import { createBrowserRouter, RouterProvider, useLoaderData as useLoaderData_ } from "react-router";

export interface LoaderData {
    config: {
        solidBg?: boolean;
        noCursor?: boolean;
    };
}

// Yes, this can violate the rules of hooks, but we don't
export const UseLoaderDataContext = namedContext<() => LoaderData>(useLoaderData_<LoaderData>, "LoaderDataContext");

// eslint-disable-next-line react-refresh/only-export-components
export function useLoaderData() {
    return useContext(UseLoaderDataContext)();
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

export function App() {
    useEffect(() => {
        installF8Break();
        return uninstallF8Break;
    }, []);
    return (
        <StrictMode>
            <RouterProvider router={router} />
        </StrictMode>
    );
}

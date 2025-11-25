import { installF8Break, namedContext, uninstallF8Break } from "@/utils/devtools";

import { LayerContext } from "./components/Layer/context";
import { assert } from "./utils/error";

import "./app.scss";
import { StrictMode, useContext, useEffect, useState } from "react";
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
                path: "vis",
                loader(): LoaderData {
                    return {
                        config: {
                            noCursor: true,
                        },
                    };
                },
                async lazy() {
                    const Component = (await import("./pages/vis")).default;

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
            {
                path: "88x31",
                async lazy() {
                    const Component = (await import("./pages/88x31")).default;

                    return {
                        Component,
                    };
                },
            },
            {
                path: "ast-viewer",
                loader(): LoaderData {
                    return {
                        config: {
                            solidBg: true,
                        },
                    };
                },
                async lazy() {
                    const Component = (await import("./pages/ast-viewer")).default;

                    return {
                        Component,
                    };
                },
            },
        ],
    },
]);

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
        <StrictMode>
            <LayerContext value={ctx}>
                <RouterProvider router={router} />
            </LayerContext>
        </StrictMode>
    );
}

import { identity } from "@/utils/functional";
import { makeLazy } from "@/utils/lazy";
import { createRouter } from "@tanstack/react-router";

// Import the generated route tree
import { routeTree } from "./routeTree.gen";

// Create a new router instance

export const getRouter = (import.meta.env.SSR ? identity : makeLazy)(() => {
    console.log("Foobar");

    const router = createRouter({
        routeTree,
        scrollRestoration: true,
        defaultPreloadStaleTime: 0,
        defaultPreload: "intent",
    });

    return router;
});

declare module "@tanstack/react-router" {
    interface StaticDataRouteOption {
        pageTitle?: string;
        description?: string;
        imageUrl?: string;
    }
}

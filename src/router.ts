import { createRouter } from "@tanstack/react-router";

import { routeTree } from "./routeTree.gen";

export function makeRouter() {
    return createRouter({
        routeTree,
        scrollRestoration: true,
        defaultPreload: "intent",
        context: {},
    });
}
declare module "@tanstack/react-router" {
    interface Register {
        router: ReturnType<typeof makeRouter>;
    }
}

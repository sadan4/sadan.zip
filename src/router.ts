import { createRouter } from "@tanstack/react-router";

import { routeTree } from "./routeTree.gen";

export function makeRouter() {
    return createRouter({
        routeTree,
        scrollRestoration: true,
        defaultPreload: "intent",
        context: {
            head: "",
        },
    });
}
declare module "@tanstack/react-router" {
    interface Register {
        router: ReturnType<typeof makeRouter>;
    }
}

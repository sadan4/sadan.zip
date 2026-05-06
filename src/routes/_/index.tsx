import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_/")({
    beforeLoad(_ctx) {
        throw redirect({
            to: "/{-$tab}",
            params: { tab: "about" },
        });
    },
    staticData: {
        pageTitle: "Home",
        description: "My silly website.",
        imageUrl: "/assets/avatar.webp",
    },
});


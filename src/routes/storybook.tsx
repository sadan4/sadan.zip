import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_layout/storybook")({
    loader() {
        throw redirect({
            href: "/storybook/index.html",
        });
    },
});

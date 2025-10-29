import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/storybook")({
    loader() {
        throw redirect({
            href: "/storybook/index.html",
        });
    },
});

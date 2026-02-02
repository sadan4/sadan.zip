import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_/88x31/")({
    beforeLoad() {
        throw redirect({
            to: "/88x31/$lang",
            params: { lang: "html" },
        });
    },
});

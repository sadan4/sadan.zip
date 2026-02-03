import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/minky")({
    component: Minky,
    staticData: {
        description: "Minky",
        pageTitle: "Minky",
        imageUrl: "/assets/minky2.jpg",
    },
    head() {
        return {
            meta: [
                {
                    property: "pg:title",
                    content: "Minky",
                },
            ],
        };
    },
});

function Minky() {
    return <div>Mink</div>;
}

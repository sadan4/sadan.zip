import { HomePage } from "@/components/HomePage";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/")({
    staticData: {
        pageTitle: "Home",
        description: "My silly website.",
        imageUrl: "/assets/avatar.webp",
    },
    component: RouteComponent,
});


function RouteComponent() {
    return <HomePage tab="about" />;
}

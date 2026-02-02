import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/minky")({
    component: Minky,
});

function Minky() {
    return <div>Mink</div>;
}

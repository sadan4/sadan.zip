import { Text } from "@/components/Text";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_layout/minky")({
    component: Mink,
});

function Mink() {
    return (
        <Text
            size="9xl"
            color="primary"
            center
        >
            Mink
        </Text>
    );
}

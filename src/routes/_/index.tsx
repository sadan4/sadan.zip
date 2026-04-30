import Avatar from "@/components/Avatar";
import { Boilerplate } from "@/components/Boilerplate";
import { TextLink } from "@/components/Links";
import Name from "@/components/Name";
import { Text } from "@/components/Text";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/")({
    component: HomePage,
    staticData: {
        pageTitle: "Home",
        description: "My silly website.",
        imageUrl: "/assets/avatar.webp",
    },
});

function HomePage() {
    return (
        <>
            <Boilerplate />
            <div className="flex flex-col items-center pt-52">
                <Avatar
                    className="h-52 ff:w-52"
                    round
                />
                <Name />
                <Text
                    color="success"
                    size="md"
                    className="mt-6 mb-4"
                >
                    Random loser on the internet.
                </Text>
                <TextLink to="/links" size="lg" color="secondary">
                    Links
                </TextLink>
            </div>
        </>
    );
}


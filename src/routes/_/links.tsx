import { Boilerplate } from "@/components/Boilerplate";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink, TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/links")({
    component: LinksPage,
    staticData: {
        pageTitle: "Links",
        description: "Where to find me on the internet.",
    },
});

function Links() {
    return (
        <div className="flex flex-wrap justify-center gap-6 [&_svg]:h-16 [&_svg]:text-secondary-500">
            <DiscordIconLink
                userId="521819891141967883"
                key="521819891141967883"
                aria-label="Discord"
            />
            <NameMCIconLink
                UUID="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
                key="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
                aria-label="NameMC"
            />
            <LastFMIconLink
                username="sadan4"
                key="lastfm-sadan4"
                aria-label="LastFM"
            />
            <SteamIconLink
                userId="sadan4"
                key="steam-sadan4"
                aria-label="Steam"
            />
            <FortniteDBIconLink
                username="sadan4"
                key="fndb-sadan4"
                aria-label="FortniteDB"
            />
            <GithubIconLink
                username="sadan4"
                key="gh-sadan4"
                aria-label="GitHub"
            />
        </div>
    );
}

function LinksPage() {
    return (
        <>
            <Boilerplate />
            <div className="flex flex-col items-center px-4 pt-52">
                <Text
                    size="xl"
                    weight="bold"
                    className="mb-12"
                >
                    Socials
                </Text>
                <Links />
                <Text
                    color="secondary"
                    size="md"
                    className="mt-12 mb-4"
                >
                    Feel free to reach out!
                </Text>
                <TextLink
                    to="/"
                    color="primary"
                >
                    Back Home
                </TextLink>
            </div>
        </>
    );
}

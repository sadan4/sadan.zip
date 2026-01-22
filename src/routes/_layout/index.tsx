import Avatar from "@/components/Avatar";
import { Boilerplate } from "@/components/Boilerplate";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink } from "@/components/Links";
import Name from "@/components/Name";
import { Text } from "@/components/Text";
import { createFileRoute } from "@tanstack/react-router";

function Links() {
    return (
        <div className="flex gap-3 [&_svg]:h-14 [&_svg]:text-secondary-500">
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

export const Route = createFileRoute("/_layout/")({
    component: HomePage,
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
                <Links />
                <Text
                    color="success"
                    size="md"
                    className="mt-6"
                >
                    Random loser on the internet.
                </Text>
            </div>
        </>
    );
}

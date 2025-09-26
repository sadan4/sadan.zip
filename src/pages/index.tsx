import { Boilerplate } from "@/components/Boilerplate";
import { Text } from "@/components/Text";

import Avatar from "../components/Avatar";
import { DefaultFooter, FooterContainer } from "../components/Footer";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink } from "../components/Links";
import Name from "../components/Name";

function Links() {
    return (
        <div className="[&_svg]:text-secondary-500 flex gap-3 [&_svg]:h-14">
            <DiscordIconLink
                userId="521819891141967883"
                key="521819891141967883"
            />
            <NameMCIconLink
                UUID="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
                key="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
            />
            <LastFMIconLink
                username="sadan4"
                key="lastfm-sadan4"
            />
            <SteamIconLink
                userId="sadan4"
                key="steam-sadan4"
            />
            <FortniteDBIconLink
                username="sadan4"
                key="fndb-sadan4"
            />
            <GithubIconLink
                username="sadan4"
                key="gh-sadan4"
            />
        </div>
    );
}

export default function App() {
    return (
        <>
            <Boilerplate />
            <div className="h-full w-full">
                <FooterContainer
                    footer={() => <DefaultFooter />}
                    className="flex justify-center"
                >
                    <div className="flex flex-col items-center pt-52">
                        <Avatar
                            className="w-52"
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
                </FooterContainer>
            </div>
        </>
    );
}

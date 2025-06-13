import Avatar from "./components/Avatar";
import Cursor from "./components/Cursor";
import BoundingCursor from "./components/Cursor/BoundingCursor";
import { DotCursor } from "./components/Cursor/DotCursor";
import { DefaultFooter, FooterContainer } from "./components/Footer";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink } from "./components/Links";
import Name from "./components/Name";

function Links() {
    return (
        <div className="flex [&_svg]:h-14 gap-3 [&_svg]:text-secondary">
            {
                [
                    (props) => (
                        <DiscordIconLink
                            userId="521819891141967883"
                            key="521819891141967883"
                            {...props}
                        />
                    ),
                    (props) => (
                        <NameMCIconLink
                            UUID="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
                            key="b7c4f5b1-762f-41ea-b6b4-45aba74198e5"
                            {...props}
                        />
                    ),
                    (props) => (
                        <LastFMIconLink
                            username="sadan4"
                            key="lastfm-sadan4"
                            {...props}
                        />
                    ),
                    (props) => (
                        <SteamIconLink
                            userId="sadan4"
                            key="steam-sadan4"
                            {...props}
                        />
                    ),
                    (props) => (
                        <FortniteDBIconLink
                            username="sadan4"
                            key="fndb-sadan4"
                            {...props}
                        />
                    ),
                    (props) => (
                        <GithubIconLink
                            username="sadan4"
                            key="gh-sadan4"
                            {...props}
                        />
                    ),
                ].map((el, _idx) => el({}))
            }
        </div>
    );
}

export default function App() {
    return (
        <>
            <Cursor>
                <DotCursor
                    className="bg-bg-fg mix-blend-exclusion z-999"
                    radius={10}
                    invert
                />
            </Cursor>
            <Cursor>
                <BoundingCursor
                    className="bg-bg-fg mix-blend-exclusion"
                    frameLength={{
                        type: "dynamic",
                    }}
                    thickness={4}
                />
            </Cursor>
            <div className="h-full w-full">
                <FooterContainer
                    footer={<DefaultFooter />}
                    className="flex justify-center"
                >
                    <div className="pt-52 flex items-center flex-col">
                        <Avatar
                            className="w-52"
                            round
                        />
                        <Name />
                        <Links />
                        <div className="text-success mt-6">
                            Random loser on the internet.
                        </div>
                    </div>
                </FooterContainer>
            </div>
        </>
    );
}

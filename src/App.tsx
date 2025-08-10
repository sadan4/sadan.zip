import cursorStyles from "@/components/Cursor/styles.module.css";

import Avatar from "./components/Avatar";
import Cursor from "./components/Cursor";
import BoundingCursor from "./components/Cursor/BoundingCursor";
import { DotCursor } from "./components/Cursor/DotCursor";
import { DefaultFooter, FooterContainer } from "./components/Footer";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink } from "./components/Links";
import ModalRenderRoot from "./components/modal/ModalRenderRoot";
import Name from "./components/Name";
import cn from "./utils/cn";

function Links() {
    return (
        <div className="flex [&_svg]:h-14 gap-3 [&_svg]:text-secondary">
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
            <Cursor>
                <DotCursor
                    className={cn("bg-bg-fg mix-blend-exclusion", cursorStyles.cursorZ)}
                    radius={10}
                    invert
                />
            </Cursor>
            <Cursor>
                <BoundingCursor
                    className={cn("bg-bg-fg mix-blend-exclusion", cursorStyles.cursorZ)}
                    frameLength={{
                        type: "dynamic",
                        factor: 1 / 10,
                        min: 8,
                        max: 25,
                    }}
                    unHoveredRadius={15}
                    thickness={4}
                    borderFocusedItems
                />
            </Cursor>
            <ModalRenderRoot />
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

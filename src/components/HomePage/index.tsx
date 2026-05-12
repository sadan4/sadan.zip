import Avatar from "@/components/Avatar";
import { Boilerplate } from "@/components/Boilerplate";
import Discord from "@/components/icons/Discord";
import Github from "@/components/icons/Github";
import LastFM from "@/components/icons/LastFM";
import NameMC from "@/components/icons/NameMC";
import SaveTheWorld from "@/components/icons/SaveTheWorld";
import Steam from "@/components/icons/Steam";
import { TabBar } from "@/components/layout/TabBar";
import { ExternalLink, TextLink } from "@/components/Links";
import Name from "@/components/Name";
import { Text } from "@/components/Text";
import cn from "@/utils/cn";
import {
    DISCORD_ID,
    discordUrl,
    EPIC_USERNAME,
    fndbProfileUrl,
    GITHUB_PROFILE_URL,
    LASTFM_USERNAME,
    lastFMProfileUrl,
    MC_UUID,
    nameMCUrl,
    STEAM_USERNAME,
    steamProfileUrl,
} from "@/utils/constants";

import * as styles from "./styles.module.scss";

import { ArrowRightIcon } from "lucide-react";
import type { ReactNode } from "react";

interface ContactLinkProps {
    name: string;
    to: string;
    icon?: ReactNode;
}

function HoverArrow() {
    return (
        <ArrowRightIcon className={cn(styles.hoverArrow)} />
    );
}

function ContactLink({ name, to, icon }: ContactLinkProps) {
    return (
        <div>
            <ExternalLink to={to}>
                <div className={cn("flex w-fit items-center gap-2", styles.hoverArrowContainer)}>
                    {icon}
                    <Text
                        size="lg"
                        color="primary"
                    >
                        {name}
                    </Text>
                    <HoverArrow />
                </div>
            </ExternalLink>
        </div>
    );
}

function LinksTabSection() {
    return (
        <div>
            <Text size="md">
                I'm most active on Discord. If you need to contact me, you should reach out there.
            </Text>
            <div className={styles.links}>
                <ContactLink
                    name="Discord"
                    icon={<Discord className="size-6" />}
                    to={discordUrl(DISCORD_ID)}
                />
            </div>
            <Text>
                You can find me on other platforms as well!
            </Text>
            <div className={cn(styles.links, "flex gap-4")}>
                <ContactLink
                    name="GitHub"
                    to={GITHUB_PROFILE_URL}
                    icon={<Github className="size-6" />}
                />
                <ContactLink
                    name="Steam"
                    to={steamProfileUrl(STEAM_USERNAME)}
                    icon={<Steam className="size-6" />}
                />
                <ContactLink
                    name="NameMC"
                    to={nameMCUrl(MC_UUID)}
                    icon={<NameMC className="size-6" />}
                />
                <ContactLink
                    name="last.fm"
                    to={lastFMProfileUrl(LASTFM_USERNAME)}
                    icon={<LastFM className="size-6" />}
                />
                <ContactLink
                    name="FNDB"
                    to={fndbProfileUrl(EPIC_USERNAME)}
                    icon={<SaveTheWorld className="size-6" />}
                />
            </div>
        </div>
    );
}

export type HomePageTab = "about" | "links";

export interface HomePageProps {
    tab: HomePageTab;
}

export function HomePage({ tab }: HomePageProps) {
    return (
        <>
            <Boilerplate />
            <div className="flex flex-col items-center pt-52">
                <Avatar
                    className="h-52 ff:w-52"
                    round
                />
                <Name />
                <div className="w-1/2">
                    <TabBar
                        selectedTab={tab}
                        tabs={[
                            {
                                id: "about",
                                render() {
                                    return (
                                        <div>
                                            <Text
                                                size="2xl"
                                                color="primary"
                                            >
                                                About Me
                                            </Text>
                                            <Text size="md">
                                                {/* eslint-disable @stylistic/max-len */}
                                                I'm a student who loves tinkering with and building software.
                                                I love web development, developer tooling, open source software and modding.
                                                In my free time, I enjoy playing video games with my friends, tinkering with hardware and reading books.
                                                {/* eslint-enable @stylistic/max-len */}
                                            </Text>
                                        </div>
                                    );
                                },
                                renderTab(_props) {
                                    return (
                                        <TextLink
                                            to="/{-$tab}"
                                            params={{ tab: "about" }}
                                            size="xl"
                                        >
                                            About
                                        </TextLink>
                                    );
                                },
                            },
                            {
                                id: "links",
                                render() {
                                    return <LinksTabSection />;
                                },
                                renderTab(_props) {
                                    return (
                                        <TextLink
                                            to="/{-$tab}"
                                            params={{ tab: "links" }}
                                            size="xl"
                                        >
                                            Links
                                        </TextLink>
                                    );
                                },
                            },
                        ]}
                    />
                </div>
            </div>
        </>
    );
}

import { Clickable } from "@/components/Clickable";
import Discord from "@/components/icons/Discord";
import LinkIcon from "@/components/icons/FriendLink";
import Circle from "@/components/layout/Circle";
import { Square } from "@/components/testing";
import { Text } from "@/components/Text";
import { discordUrl } from "@/utils/constants";
import type { Friend } from "@/utils/friends";

import { Fragment } from "react";


export interface FriendCardProps {
    friend: Friend;
}

export default function FriendCard({ friend }: FriendCardProps) {
    return (
        <Circle
            radius={192}
            numItems={4}
        >
            <Fragment key="url">
                {
                    friend.url
                        ? (
                            <Clickable
                                tag="a"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="bg-bg-100 flex h-12 w-12 items-center justify-center rounded-full"
                                href={friend.url.toString()}
                            >
                                <LinkIcon className="h-9 w-9" />
                            </Clickable>
                        )
                        : (
                            <Clickable
                                tag="a"
                                className="bg-bg-100 flex h-12 w-12 cursor-not-allowed items-center justify-center rounded-full brightness-50"
                                onClick={(e) => {
                                    e.preventDefault();
                                }}
                            >
                                <LinkIcon className="h-9 w-9" />
                            </Clickable>
                        )
                }
            </Fragment>
            <Text
                color="info"
                size="3xl"
                key="name"
                className="px-2"
            >
                {friend.name}
            </Text>
            <Fragment key="discord">
                {
                    friend.discordId
                        ? (
                            <Clickable
                                tag="a"
                                href={discordUrl(friend.discordId)
                                    .toString()}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="bg-bg-100 flex h-12 w-12 items-center justify-center rounded-full"
                            >
                                <Discord className="h-8 w-8" />
                            </Clickable>
                        )
                        : (
                            <>
                                <Square />
                            </>
                        )
                }
            </Fragment>
        </Circle>
    );
}

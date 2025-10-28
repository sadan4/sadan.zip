import { Clickable } from "@/components/Clickable";
import Discord from "@/components/icons/Discord";
import { LinkIcon } from "@/components/icons/Link";
import { Circle } from "@/components/layout/Circle";
import { Square } from "@/components/testing";
import { Text } from "@/components/Text";
import { discordUrl } from "@/utils/constants";
import type { Friend } from "@/utils/friends";

import { Fragment } from "react";


export interface FriendCardProps {
    friend: Friend;
}


function C88X31({ friend }: FriendCardProps) {
    return (
        <Clickable
            tag="a"
            href={friend.url?.toString() ?? "#"}
            target="_blank"
            rel="noopener noreferrer"
            className="block h-[31px] w-[88px]"
        >
            <img
                className="h-[31px] w-[88px] [image-rendering:pixelated]"
                src={friend._88x31url?.toString()}
                alt={`${friend.name} 88x31 banner`}
            />
        </Clickable>
    );
}

export default function FriendCard({ friend }: FriendCardProps) {
    return (
        <Circle.Items
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
                                className="flex h-12 w-12 items-center justify-center rounded-full bg-bg-100"
                                href={friend.url.toString()}
                            >
                                <LinkIcon
                                    className="h-9 w-9"
                                />
                            </Clickable>
                        )
                        : (
                            <Clickable
                                tag="a"
                                className="flex h-12 w-12 cursor-not-allowed items-center justify-center rounded-full bg-bg-100 brightness-50"
                                onClick={(e) => {
                                    e.preventDefault();
                                }}
                            >
                                <LinkIcon className="h-9 w-9" />
                            </Clickable>
                        )
                }
            </Fragment>
            {
                friend._88x31url
                    ? <C88X31 friend={friend} />
                    : (
                        <Text
                            color="info-400"
                            size="3xl"
                            key="name"
                            className="px-2"
                        >
                            {friend.name}
                        </Text>
                    )
            }
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
                                className="flex h-12 w-12 items-center justify-center rounded-full bg-bg-100"
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
        </Circle.Items>
    );
}

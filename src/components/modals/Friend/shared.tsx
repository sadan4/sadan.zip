import { Clickable } from "@/components/Clickable";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import Discord from "@/components/icons/Discord";
import { LinkIcon } from "@/components/icons/Link";
import { CircleCenter, CircleItems, CircleRoot } from "@/components/layout/Circle";
import { PopoutContent, PopoutRoot, PopoutTrigger } from "@/components/Popout";
import { Position } from "@/components/Popout/enums";
import { Text } from "@/components/Text";
import { Tooltip } from "@/components/Tooltip";
import type { TooltipPosition } from "@/components/Tooltip/constants";
import { useRect } from "@/hooks/rect";
import { discordUrl } from "@/utils/constants";
import { error } from "@/utils/error";
import type { Friend } from "@/utils/friends";

import { FRIEND_CARD_CIRCLE_DIAMETER } from "./other";

import { Fragment, useId, useState } from "react";
import { preload } from "react-dom";

interface FriendCardProps {
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

function FriendCard({ friend }: FriendCardProps) {
    return (
        <CircleItems
            diameter={FRIEND_CARD_CIRCLE_DIAMETER}
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
                        : error()
                }
            </Fragment>
        </CircleItems>
    );
}


export interface FriendButtonProps {
    friend: Friend;
    tooltipPosition: TooltipPosition;
    mobile?: true;
}

export function FriendButton({ friend, tooltipPosition, mobile }: FriendButtonProps) {
    const [popoutOpen, setPopoutOpen] = useState(false);
    const [tooltipVisible, setTooltipVisible] = useState(false);
    const [el, setEl] = useState<HTMLElement | null>(null);
    const rect = useRect(el);
    const maskId = useId();
    const gradientId = useId();

    return (
        <PopoutRoot
            onOpen={() => {
                setPopoutOpen(true);
            }}
            onClose={() => {
                setPopoutOpen(false);
            }}
        >
            <CircleRoot>
                <CircleCenter>
                    <PopoutTrigger>
                        <Tooltip
                            position={tooltipPosition}
                            text={friend.name}
                            show={tooltipVisible && !popoutOpen}
                            onShow={() => {
                                setTooltipVisible(true);
                            }}
                            onHide={() => {
                                setTooltipVisible(false);
                            }}
                        >
                            <Clickable
                                // avoid calculations when not needed
                                ref={mobile && setEl}
                                onMouseOver={() => {
                                    if (friend._88x31url) {
                                        preload(friend._88x31url.toString(), { as: "image" });
                                    }
                                }}
                            >
                                <PerspectiveHover
                                    hoverFactor={2}
                                >
                                    <Shadow>
                                        <img
                                            src={friend.avatarUrl.toString()}
                                            // cursed, but doesn't work otherwise
                                            className="h-24 max-h-24 min-h-24 w-24 max-w-24 min-w-24 rounded-full select-none"
                                        />
                                    </Shadow>
                                </PerspectiveHover>
                            </Clickable>
                        </Tooltip>
                    </PopoutTrigger>
                </CircleCenter>
                <PopoutContent
                    position={Position.CENTER}
                    onDismiss={() => {
                        setTooltipVisible(false);
                    }}
                >
                    {
                        mobile && rect && popoutOpen && (
                            <svg
                                className="fixed overflow-visible"
                                style={{
                                    width: FRIEND_CARD_CIRCLE_DIAMETER,
                                    height: FRIEND_CARD_CIRCLE_DIAMETER,
                                    top: rect.top + (rect.height / 2) - (FRIEND_CARD_CIRCLE_DIAMETER / 2),
                                    left: rect.left + (rect.width / 2) - (FRIEND_CARD_CIRCLE_DIAMETER / 2),
                                }}
                            >
                                <radialGradient
                                    id={gradientId}
                                    cx="50%"
                                    cy="50%"
                                    className="text-bg-100 *:[stop-color:currentColor]"
                                >
                                    <stop
                                        offset="0%"
                                        stopOpacity={1}
                                    />
                                    <stop
                                        offset={`${(FRIEND_CARD_CIRCLE_DIAMETER * (1.3 / 2) / FRIEND_CARD_CIRCLE_DIAMETER) * 100}%`}
                                        stopOpacity={1}
                                    />
                                    <stop
                                        offset="100%"
                                        stopOpacity={0}
                                    />
                                </radialGradient>
                                <mask
                                    id={maskId}
                                    mask-type="alpha"
                                >
                                    <circle
                                        className="fill-black"
                                        r={rect.width / 2}
                                        cx="50%"
                                        cy="50%"
                                    />
                                </mask>
                                <circle
                                    className="mask-exclude"
                                    style={{
                                        // for mask-composite to work
                                        maskImage: `url(#${maskId}), linear-gradient(#000 0 0)`,
                                    }}
                                    cx="50%"
                                    cy="50%"
                                    r={FRIEND_CARD_CIRCLE_DIAMETER}
                                    fill={`url(#${gradientId})`}
                                />
                            </svg>
                        )
                    }
                    <FriendCard
                        friend={friend}
                    />
                </PopoutContent>
            </CircleRoot>
        </PopoutRoot>
    );
}

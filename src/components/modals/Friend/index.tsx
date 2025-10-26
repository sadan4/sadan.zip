import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import Circle, { DefaultPlacementCircleItem } from "@/components/layout/Circle";
import { ModalContext } from "@/components/modal";
import { Popout } from "@/components/Popout";
import { PopoutDirection } from "@/components/Popout/constants";
import { Tooltip } from "@/components/Tooltip";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { useMediaQuery } from "@/hooks/mediaQuery";
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { type Friend, friends } from "@/utils/friends";
import toCSS from "@/utils/toCSS";

import { FriendModalContext } from "./context";
import FriendCard from "./FriendCard";

import { ArrowLeftIcon, ArrowRightIcon, XIcon } from "lucide-react";
import { use, useMemo, useState } from "react";
import { preload } from "react-dom";


function FriendModalCloseIcon() {
    const ctx = use(ModalContext);

    return (
        <HoverScale factor={0.9}>
            <div className="flex h-52 w-52 items-center justify-center">
                <Clickable
                    onClick={() => {
                        ctx?.requestClose();
                    }}
                >
                    <div
                        className={cn("flex h-44 w-44 items-center justify-center rounded-full bg-bg-100")}
                    >
                        <XIcon
                            className="h-full w-full text-info-500"
                        />
                    </div>
                </Clickable>
            </div>
        </HoverScale>
    );
}

interface ArrowButtonProps {
    direction: "next" | "previous";
}

function ArrowButton({ direction }: ArrowButtonProps) {
    const Component = direction === "next" ? ArrowRightIcon : ArrowLeftIcon;

    return (
        <div>
            <Component className="h-24 w-24" />
        </div>
    );
}

interface FriendButtonProps {
    friend: Friend;
    tooltipPosition: TooltipPosition;
}

function FriendButton({ friend, tooltipPosition }: FriendButtonProps) {
    const [popoutOpen, setPopoutOpen] = useState(false);
    const [tooltipVisible, setTooltipVisible] = useState(false);

    return (
        <Popout
            side={PopoutDirection.CENTER}
            renderPopout={() => {
                return (
                    <div>
                        <FriendCard
                            friend={friend}
                        />
                    </div>
                );
            }}
            onOpen={() => {
                setPopoutOpen(true);
            }}
            onClose={() => {
                setPopoutOpen(false);
                setTooltipVisible(false);
            }}
            className="h-24 max-h-24 w-24 max-w-24"
        >
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
                <Clickable onMouseOver={() => {
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
                                className="max-h-24 max-w-24 rounded-full select-none"
                            />
                        </Shadow>
                    </PerspectiveHover>
                </Clickable>
            </Tooltip>
        </Popout>
    );
}

export default function FriendModal() {
    const isNormalScreen = useMediaQuery("(width >= 735px)");

    if (isNormalScreen) {
        return <FriendModalNormal />;
    }
    return <FriendModalMobile />;
}

function FriendModalMobile() {
    return (
        <div className="fixed inset-x-1/5 inset-y-1/8 bg-green-500/10">
            <div className="m-2 grid grid-flow-row-dense grid-cols-[repeat(auto-fill,--spacing(24))] justify-center gap-4">
                {friends.map((friend) => {
                    return (
                        <FriendButton
                            friend={friend}
                            tooltipPosition={TooltipPosition.TOP}
                        />
                    );
                })}
            </div>
        </div>
    );
}

function FriendModalNormal() {
    const center = use(FriendModalContext);
    const x = center.x + (center.width / 2);
    const y = center.y + (center.height / 2);
    const [friendIndex, setFriendIndex] = useState(0);

    const nextButton = useMemo(() => {
        return (
            <DefaultPlacementCircleItem key="next-button">
                <Clickable onClick={() => {
                    setFriendIndex((prev) => prev + 1);
                }}
                >
                    <ArrowButton direction="next" />
                </Clickable>
            </DefaultPlacementCircleItem>
        );
    }, []);

    const prevButton = useMemo(() => {
        return (
            <DefaultPlacementCircleItem key="prev-button">
                <Clickable onClick={() => {
                    setFriendIndex((prev) => prev - 1);
                }}
                >
                    <ArrowButton direction="previous" />
                </Clickable>
            </DefaultPlacementCircleItem>
        );
    }, []);

    const contents = useMemo(() => {
        return loopArrayStartingAt(friends, friendIndex)
            .slice(0, 4)
            .map((friend, idx) => {
                return (
                    <DefaultPlacementCircleItem key={friend.name}>
                        <FriendButton
                            friend={friend}
                            tooltipPosition={idx < 2 ? TooltipPosition.LEFT : TooltipPosition.RIGHT}
                        />
                    </DefaultPlacementCircleItem>
                );
            })
            .toSpliced(0, 0, nextButton, prevButton);
    }, [friendIndex, nextButton, prevButton]);

    return (
        <div
            className="fixed top-0 left-0 h-full w-full"
            onWheel={(e) => {
                console.log("scroll", e);
            }}
        >
            <div
                className="absolute -translate-1/2"
                style={{
                    top: toCSS.px(y),
                    left: toCSS.px(x),
                }}
            >
                <div
                    className="absolute top-0 left-0 h-52 w-52 -translate-1/2"
                >
                    <FriendModalCloseIcon />
                </div>
                <Circle
                    radius={500}
                    children={contents}
                    offset={1}
                />
            </div>
        </div>
    );
}

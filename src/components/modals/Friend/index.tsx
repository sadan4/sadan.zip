import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import { Circle } from "@/components/layout/Circle";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { ModalContext } from "@/components/modal";
import { Popout2 } from "@/components/Popout2";
import { Tooltip } from "@/components/Tooltip";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { useMediaQuery } from "@/hooks/mediaQuery";
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { type Friend, friends } from "@/utils/friends";

import { FriendModalContext } from "./context";
import FriendCard from "./FriendCard";
import styles from "./styles.module.scss";

import { ArrowLeftIcon, ArrowRightIcon, XIcon } from "lucide-react";
import { use, useMemo, useState } from "react";
import { preload } from "react-dom";

declare module "react" {
    interface CSSProperties {
        "--center-top"?: number;
        "--center-left"?: number;
        "--center-width"?: number;
        "--center-height"?: number;
    }
}
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
        <Popout2.Root
            onOpen={() => {
                setPopoutOpen(true);
            }}
            onClose={() => {
                setPopoutOpen(false);
            }}
        >
            <Circle.Root>
                <Circle.Center>
                    <Popout2.Trigger>
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
                    </Popout2.Trigger>
                </Circle.Center>
                <Popout2.Content
                    position={Popout2.Position.CENTER}
                    onDismiss={() => {
                        setTooltipVisible(false);
                    }}
                >
                    <FriendCard
                        friend={friend}
                    />
                </Popout2.Content>
            </Circle.Root>
        </Popout2.Root>
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
        <div className="fixed inset-x-1/5 inset-y-1/8 bg-bg-100/25">
            <ScrollArea className="max-h-full">
                <div className="m-2 grid grid-flow-row-dense grid-cols-[repeat(auto-fill,--spacing(24))] justify-center gap-4">
                    {friends.map((friend) => {
                        return (
                            <FriendButton
                                friend={friend}
                                tooltipPosition={TooltipPosition.TOP}
                                key={friend.name}
                            />
                        );
                    })}
                </div>
            </ScrollArea>

        </div>
    );
}

function FriendModalNormal() {
    const center = use(FriendModalContext);
    const [friendIndex, setFriendIndex] = useState(0);

    const nextButton = useMemo(() => {
        return (
            <Circle.DefaultPlacementCircleItem key="next-button">
                <Clickable onClick={() => {
                    setFriendIndex((prev) => prev + 1);
                }}
                >
                    <ArrowButton direction="next" />
                </Clickable>
            </Circle.DefaultPlacementCircleItem>
        );
    }, []);

    const prevButton = useMemo(() => {
        return (
            <Circle.DefaultPlacementCircleItem key="prev-button">
                <Clickable onClick={() => {
                    setFriendIndex((prev) => prev - 1);
                }}
                >
                    <ArrowButton direction="previous" />
                </Clickable>
            </Circle.DefaultPlacementCircleItem>
        );
    }, []);

    const contents = useMemo(() => {
        return loopArrayStartingAt(friends, friendIndex)
            .slice(0, 4)
            .map((friend, idx) => {
                return (
                    <Circle.DefaultPlacementCircleItem key={friend.name}>
                        <FriendButton
                            friend={friend}
                            tooltipPosition={idx < 2 ? TooltipPosition.LEFT : TooltipPosition.RIGHT}
                        />
                    </Circle.DefaultPlacementCircleItem>
                );
            })
            .toSpliced(0, 0, nextButton, prevButton);
    }, [friendIndex, nextButton, prevButton]);

    return (
        <>

            <div
                className="fixed inset-fill"
                onWheel={(e) => {
                    console.log("scroll", e);
                }}
            >
                <Circle.Root>
                    <Circle.Center>
                        <div
                            className={styles.closeIcon}
                            style={{
                                "--center-top": center?.y,
                                "--center-left": center?.x,
                                "--center-width": center?.width,
                                "--center-height": center?.height,
                            }}
                        >
                            <FriendModalCloseIcon />
                        </div>
                    </Circle.Center>
                    <Circle.Items
                        radius={500}
                        children={contents}
                        offset={1}
                    />
                </Circle.Root>
            </div>
        </>
    );
}

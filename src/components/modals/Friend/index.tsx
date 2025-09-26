import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import CloseIcon from "@/components/icons/Close";
import LeftArrow from "@/components/icons/LeftArrow";
import RightArrow from "@/components/icons/RightArrow";
import Circle, { DefaultPlacementCircleItem } from "@/components/layout/Circle";
import { useModalStackStore } from "@/components/modal/internal/modalStackStore";
import { Popout } from "@/components/Popout";
import { PopoutDirection } from "@/components/Popout/constants";
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";
import toCSS from "@/utils/toCSS";

import FriendCard from "./FriendCard";
import { defaultPosition, useFriendModalCenterStore } from "./friendModalCenterStore";

import { useMemo, useState } from "react";

export interface FriendModalProps {
}

function FriendModalCloseIcon() {
    return (
        <HoverScale factor={0.9}>
            <div className="flex h-52 w-52 items-center justify-center">
                <Clickable
                    onClick={() => {
                        useModalStackStore.getState()
                            .popModal();
                    }}
                >
                    <div
                        className={cn("bg-bg-100 flex h-44 w-44 items-center justify-center rounded-full")}
                    >
                        <CloseIcon
                            className="fill-info-700 h-2/3 w-2/3"
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
    const Component = direction === "next" ? RightArrow : LeftArrow;

    return (
        <div>
            <Component className="h-24 w-24" />
        </div>
    );
}

export default function FriendModal() {
    const { x, y } = useFriendModalCenterStore((x) => x.pos) ?? defaultPosition();
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
            .map((friend) => (
                <DefaultPlacementCircleItem key={friend.name}>
                    <Popout
                        side={PopoutDirection.CENTER}
                        renderPopout={() => {
                            return (
                                <div>
                                    <FriendCard friend={friend} />
                                </div>
                            );
                        }}
                    >
                        <Clickable>
                            <PerspectiveHover hoverFactor={2}>
                                <Shadow>
                                    <img
                                        src={friend.avatarUrl.toString()}
                                        className="h-24 min-h-24 w-24 min-w-24 rounded-full select-none"
                                    />
                                </Shadow>
                            </PerspectiveHover>
                        </Clickable>
                    </Popout>
                </DefaultPlacementCircleItem>
            ))
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

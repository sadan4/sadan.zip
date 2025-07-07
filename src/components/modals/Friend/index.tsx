import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import CloseIcon from "@/components/icons/Close";
import LeftArrow from "@/components/icons/LeftArrow";
import RightArrow from "@/components/icons/RightArrow";
import Circle, { type CircleItemProps, DefaultPlacementCircleItem } from "@/components/layout/Circle";
import { useModalStackStore } from "@/components/modal/internal/modalStackStore";
import { Popout } from "@/components/Popout";
import { PopoutDirection } from "@/components/Popout/constants";
import { Square } from "@/components/testing";
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";
import toCSS from "@/utils/toCSS";

import { defaultPosition, useFriendModalCenterStore } from "./friendModalCenterStore";

import __ from "lodash/fp";
import { Fragment, useMemo, useState } from "react";

export interface FriendModalProps {
}

function FriendModalCloseIcon() {
    return (
        <HoverScale factor={0.9}>
            <div className="flex items-center justify-center w-52 h-52">
                <Clickable
                    onClick={() => {
                        useModalStackStore.getState()
                            .popModal();
                    }}
                >
                    <div
                        className={cn("w-44 h-44 rounded-full bg-bg-100 flex items-center justify-center")}
                    >
                        <CloseIcon
                            className="w-2/3 h-2/3 fill-info-700"
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
            <Component className="w-24 h-24" />
        </div>
    );
}

export default function FriendModal() {
    const { x, y } = useFriendModalCenterStore(__.prop("pos")) ?? defaultPosition();
    const [friendIndex, setFriendIndex] = useState(0);

    const nextButton = useMemo(() => {
        return (
            (props: CircleItemProps) => (
                <Fragment key="next-button">
                    <DefaultPlacementCircleItem {...props}>
                        <Clickable onClick={() => {
                            setFriendIndex((prev) => prev + 1);
                        }}
                        >
                            <ArrowButton direction="next" />
                        </Clickable>
                    </DefaultPlacementCircleItem>
                </Fragment>
            )
        );
    }, []);

    const prevButton = useMemo(() => {
        return (
            (props: CircleItemProps) => (
                <Fragment key="prev-button">
                    <DefaultPlacementCircleItem
                        {...props}
                    >
                        <Clickable onClick={() => {
                            setFriendIndex((prev) => prev - 1);
                        }}
                        >
                            <ArrowButton direction="previous" />
                        </Clickable>
                    </DefaultPlacementCircleItem>
                </Fragment>
            )
        );
    }, []);

    const contents = useMemo(() => {
        return loopArrayStartingAt(friends, friendIndex)
            .slice(0, 4)
            .map((friend) => (props: CircleItemProps) => (
                <Fragment key={friend.name}>
                    <DefaultPlacementCircleItem {...props}>
                        <Popout
                            side={PopoutDirection.CENTER}
                            renderPopout={() => {
                                return (
                                    <div className="">
                                        <Square />
                                    </div>
                                );
                            }}
                        >
                            <PerspectiveHover hoverFactor={2}>
                                <Shadow>
                                    <img
                                        src={friend.avatarUrl.toString()}
                                        className="h-24 w-24 min-w-24 min-h-24 rounded-full select-none"
                                    />
                                </Shadow>
                            </PerspectiveHover>
                        </Popout>
                    </DefaultPlacementCircleItem>
                </Fragment>
            ))
            .toSpliced(0, 0, nextButton, prevButton);
    }, [friendIndex, nextButton, prevButton]);

    return (
        <div className="top-0 left-0 fixed w-full h-full z-101">
            <div
                className="absolute -translate-1/2"
                style={{
                    top: toCSS.px(y),
                    left: toCSS.px(x),
                }}
            >
                <div
                    className="absolute w-52 h-52 top-0 left-0 -translate-1/2"
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

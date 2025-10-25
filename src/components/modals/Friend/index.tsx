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
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { measureRect } from "@/utils/dom";
import { friends } from "@/utils/friends";
import toCSS from "@/utils/toCSS";
import useResizeObserver from "@react-hook/resize-observer";

import { FriendModalContext } from "./context";
import FriendCard from "./FriendCard";

import { ArrowLeftIcon, ArrowRightIcon, XIcon } from "lucide-react";
import { use, useCallback, useEffect, useMemo, useState } from "react";
import { preload } from "react-dom";

function defaultPosition() {
    return {
        x: window.innerWidth / 2,
        y: window.innerHeight / 2,
    };
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

export default function FriendModal() {
    const center = use(FriendModalContext).centerElement;
    const [{ x, y }, setCoords] = useState(defaultPosition);

    const updateCoords = useCallback(() => {
        if (center.current) {
            const { x, y, width, height } = measureRect(center.current);

            setCoords({
                x: x + (width / 2),
                y: y + (height / 2),
            });
        }
    }, [center]);

    useResizeObserver(center, updateCoords);

    useEffect(updateCoords, [updateCoords]);

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
                function Render() {
                    const [popoutOpen, setPopoutOpen] = useState(false);
                    const [tooltipVisible, setTooltipVisible] = useState(false);

                    return (
                        <DefaultPlacementCircleItem key={friend.name}>
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
                            >
                                <Tooltip
                                    position={idx < 2 ? TooltipPosition.LEFT : TooltipPosition.RIGHT}
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
                                        <PerspectiveHover hoverFactor={2}>
                                            <Shadow>
                                                <img
                                                    src={friend.avatarUrl.toString()}
                                                    className="h-24 min-h-24 w-24 min-w-24 rounded-full select-none"
                                                />
                                            </Shadow>
                                        </PerspectiveHover>
                                    </Clickable>
                                </Tooltip>
                            </Popout>
                        </DefaultPlacementCircleItem>
                    );
                }
                return <Render />;
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

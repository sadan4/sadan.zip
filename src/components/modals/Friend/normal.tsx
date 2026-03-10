import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import { CircleCenter, CircleItems, CircleRoot, DefaultPlacementCircleItem } from "@/components/layout/Circle";
import { ModalContext } from "@/components/modal";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { loopArrayStartingAt } from "@/utils/array";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";

import { FriendModalContext, NORMAL_MAIN_CIRCLE_DIAMETER } from "./other";
import { FriendButton } from "./shared";
import styles from "./styles.module.scss";

import { ArrowLeftIcon, ArrowRightIcon, XIcon } from "lucide-react";
import { use, useMemo, useState } from "react";

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

export function FriendModalNormal() {
    const center = use(FriendModalContext);
    const [friendIndex, setFriendIndex] = useState(0);

    const nextButton = useMemo(() => {
        return (
            <DefaultPlacementCircleItem key="next-button">
                <Clickable onClick={() => {
                    setFriendIndex((prev) => prev - 1);
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
                    setFriendIndex((prev) => prev + 1);
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
                // index is an important key because of positioning
                // FIXME: is index really needed
                return (
                    // eslint-disable-next-line @eslint-react/no-array-index-key
                    <DefaultPlacementCircleItem key={`${idx}-${friend.name}`}>
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
            className="flex items-center justify-center overflow-clip"
        >
            <CircleRoot>
                <CircleCenter>
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
                </CircleCenter>
                <CircleItems
                    diameter={NORMAL_MAIN_CIRCLE_DIAMETER}
                    offset={1}
                >
                    {contents}
                </CircleItems>
            </CircleRoot>
        </div>
    );
}

import { Clickable } from "@/components/Clickable";
import HoverScale from "@/components/effects/HoverScale";
import CloseIcon from "@/components/icons/Close";
import Circle from "@/components/layout/Circle";
import { useModalStackStore } from "@/components/modal/internal/modalStackStore";
import { Square } from "@/components/testing";
import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";

import { defaultPosition, useFriendModalCenterStore } from "./friendModalCenterStore";

import __ from "lodash/fp";

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

export default function FriendModal() {
    const { x, y } = useFriendModalCenterStore(__.prop("pos")) ?? defaultPosition();

    return (
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
                numItems={6}
                offset={3}
                children={Array.from({ length: 4 }, () => ({ x, y }) => (
                    <Square
                        style={{
                            top: toCSS.px(y),
                            left: toCSS.px(x),
                        }}
                    />
                ))}
            />
        </div>
    );
}

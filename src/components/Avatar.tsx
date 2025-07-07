import avatar from "@/assets/avatar.webp";
import BorderHoldCircular from "@/components/effects/BorderHold/Circular";
import { useSize } from "@/hooks/size";
import cn from "@/utils/cn";

import PerspectiveHover from "./effects/PerspectiveHover";
import Shadow from "./effects/Shadow";
import FriendModal from "./modals/Friend";
import { useFriendModalCenterStore } from "./modals/Friend/friendModalCenterStore";
import { openModal } from "./modal";

import { type ComponentProps, useEffect, useRef } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

export default function Avatar({ round = false, ...props }: AvatarProps) {
    const imgRef = useRef<HTMLImageElement>(null);
    const { x, y, width, height } = useSize(() => imgRef.current) ?? {};

    useEffect(() => {
        if (x === undefined || y === undefined || width === undefined || height === undefined) {
            useFriendModalCenterStore
                .getState()
                .resetPosition();
        } else {
            useFriendModalCenterStore
                .getState()
                .updateFromPosition(x + (width / 2), y + (height / 2));
        }
    }, [x, y, width, height]);

    return (
        <PerspectiveHover hoverFactor={4}>
            <Shadow>
                <BorderHoldCircular onHold={() => {
                    openModal({
                        key: "MODAL_FRIENDS",
                        render() {
                            return <FriendModal />;
                        },
                    });
                }}
                >
                    <img
                        ref={imgRef}
                        src={avatar}
                        alt="my discord profile picture, imagine a cute cat!"
                        {...props}
                        className={cn("max-w-sm max-h-max select-none", round && "rounded-full", props.className)}
                        draggable={false}
                    />
                </BorderHoldCircular>
            </Shadow>
        </PerspectiveHover>
    );
}

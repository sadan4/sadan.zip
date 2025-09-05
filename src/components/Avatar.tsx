import avatar from "@/assets/avatar.webp";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";
import { once } from "@/utils/functional";
import { Clickable } from "@components/Clickable";
import { openModal } from "@components/modal";
import { useFriendModalCenterStore } from "@components/modals/Friend/friendModalCenterStore";
import BorderHoldCircular from "@effects/BorderHold/Circular";
import PerspectiveHover from "@effects/PerspectiveHover";
import Shadow from "@effects/Shadow";
import { useSize } from "@hooks/size";

import { type ComponentProps, useEffect, useRef } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

const preloadFriends = once(function preloadFriends() {
    for (const { avatarUrl } of friends) {
        const img = new Image();

        img.src = avatarUrl.toString();
    }
});


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
        <Clickable>
            <PerspectiveHover hoverFactor={4}>
                <Shadow>
                    <BorderHoldCircular onHold={async () => {
                        const FriendModal = (await import("@/components/modals/Friend")).default;

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
                            onMouseOver={preloadFriends}
                            className={cn("max-w-sm max-h-max select-none", round && "rounded-full", props.className)}
                            draggable={false}
                        />
                    </BorderHoldCircular>
                </Shadow>
            </PerspectiveHover>
        </Clickable>
    );
}

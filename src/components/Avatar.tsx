import avatar from "@/assets/avatar.webp";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";
import { once } from "@/utils/functional";
import { Clickable } from "@components/Clickable";
import { BorderHoldRounded } from "@effects/BorderHold";
import PerspectiveHover from "@effects/PerspectiveHover";
import Shadow from "@effects/Shadow";

import { FriendModalContext } from "./modals/Friend/context";
import { Modal, type ModalContext } from "./modal";

import { type ComponentProps, lazy, useRef } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

const preloadFriends = once(function preloadFriends() {
    for (const { avatarUrl } of friends) {
        const img = new Image();

        img.src = avatarUrl.toString();
    }
});

const FriendModal = lazy(() => import("@components/modals/Friend"));

export default function Avatar({ round = false, ...props }: AvatarProps) {
    const modal = useRef<ModalContext>(null);
    const imgRef = useRef<HTMLImageElement>(null);

    return (
        <Clickable>
            <PerspectiveHover hoverFactor={4}>
                <Shadow>
                    <BorderHoldRounded onHold={() => {
                        modal.current?.open();
                    }}
                    >
                        <img
                            ref={imgRef}
                            src={avatar}
                            alt="my discord profile picture, imagine a cute cat!"
                            {...props}
                            onMouseOver={preloadFriends}
                            className={cn("max-h-max max-w-sm select-none", round && "rounded-full", props.className)}
                            draggable={false}
                        />
                    </BorderHoldRounded>
                </Shadow>
            </PerspectiveHover>
            <Modal ref={modal}>
                <FriendModalContext value={{ centerElement: imgRef }}>
                    <FriendModal />
                </FriendModalContext>
            </Modal>
        </Clickable>
    );
}

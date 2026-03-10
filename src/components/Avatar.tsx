import avatar from "@/assets/avatar.webp";
import { Clickable } from "@/components/Clickable";
import { BorderHoldRounded } from "@/components/effects/BorderHold";
import PerspectiveHover from "@/components/effects/PerspectiveHover";
import Shadow from "@/components/effects/Shadow";
import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import { friends } from "@/utils/friends";
import { once } from "@/utils/functional";
import { makeLazy, proxyLazy } from "@/utils/lazy";

import { FriendModal } from "./modals/Friend";
import { defaultPosition, FriendModalContext } from "./modals/Friend/other";
import { Modal, type ModalContext } from "./modal";

import { type ComponentProps, useMemo, useRef, useState } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

const preloadFriends = once(function preloadFriends() {
    for (const { avatarUrl } of friends) {
        const img = new Image();

        img.src = avatarUrl;
    }
});

const defaultPositionProxy = makeLazy(() => proxyLazy(defaultPosition));

export default function Avatar({ round = false, ...props }: AvatarProps) {
    const modalRef = useRef<ModalContext>(null);
    const [img, setImg] = useState<HTMLDivElement | null>(null);
    // update the rect before we open the modal to ensure the correct position;
    const _rect = useRect(img);

    const value = useMemo(() => (_rect
        ? {
            x: _rect.left,
            y: _rect.top,
            width: _rect.width,
            height: _rect.height,
        }
        : defaultPositionProxy()), [_rect]);

    return (
        // put the ref on Clicable because it's before all the effects that might change the size/position
        <Clickable ref={setImg}>
            <PerspectiveHover
                hoverFactor={4}
                className="touch-none"
            >
                <Shadow>
                    <BorderHoldRounded onHold={() => {
                        modalRef.current?.open();
                    }}
                    >
                        <img
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
            <Modal ref={modalRef}>
                <FriendModalContext value={value}>
                    <FriendModal />
                </FriendModalContext>
            </Modal>
        </Clickable>
    );
}

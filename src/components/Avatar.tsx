import avatar from "@/assets/avatar.webp";
import cn from "@/utils/cn";

import BorderHoldCircular from "./effects/borderHold/Circular";
import PerspectiveHover from "./effects/PerspectiveHover";
import Shadow from "./effects/Shadow";

import type { ComponentProps } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

export default function Avatar({ round = false, ...props }: AvatarProps) {
    return (
        <PerspectiveHover hoverFactor={4}>
            <Shadow>
                <BorderHoldCircular onHold={alert.bind(null, "Button held")}>
                    <img
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

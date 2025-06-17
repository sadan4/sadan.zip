import avatar from "@/assets/avatar.webp";
import cn from "@/utils/cn";

import PerspectiveHover from "./effects/PerspectiveHover";

import type { ComponentProps } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

export default function Avatar({ round = false, ...props }: AvatarProps) {
    return (
        <PerspectiveHover hoverFactor={4}>
            <img
                src={avatar}
                alt="my discord profile picture, imagine a cute cat!"
                {...props}
                className={cn("max-w-sm max-h-max", round && "rounded-full", props.className)}
            />
        </PerspectiveHover>
    );
}

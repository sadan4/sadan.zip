import avatar from "@/assets/avatar.webp";
import cn from "@/utils/cn";

import PerspectiveHover from "./effects/PerspectiveHover";

import type { ComponentProps } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    round?: boolean;
}

export default function Avatar({ round = false, ...props }: AvatarProps) {
    return (
        <PerspectiveHover
            shineClassName={cn(round && "rounded-full overflow-clip")}
        >
            {
                (hoverProps) => (
                    <img
                        src={avatar}
                        alt="my discord profile picture, imagine a cute cat!"
                        {...props}
                        {...hoverProps}
                        className={cn("max-w-sm max-h-max", round && "rounded-full", props.className, hoverProps.className)}
                    />
                )
            }
        </PerspectiveHover>
    );
}

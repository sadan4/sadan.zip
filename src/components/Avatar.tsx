import avatar from "@/assets/avatar.webp";

import Shine from "./effects/Shine";

import type { ComponentProps } from "react";

export interface AvatarProps extends ComponentProps<"img"> {
    maskClassName?: string;
}

export default function Avatar({ maskClassName = "", ...props }: AvatarProps) {
    return (
        <Shine maskClassName={maskClassName}>
            <img
                src={avatar}
                alt="my discord profile picture, imagine a cute cat!"
                {...props}
                className={`max-w-sm max-h-max ${props.className ?? ""} ${maskClassName}`}
            />
        </Shine>
    );
}

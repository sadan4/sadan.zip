import avatar from "@/assets/avatar.webp";

import PerspectiveHover from "./effects/PerspectiveHover";

import type { ComponentProps } from "react";

export default function Avatar(props: ComponentProps<"img">) {
    return (
        <PerspectiveHover className="rounded-full">
            <img
                className="rounded-full max-w-sm max-h-max w-auto h-auto"
                src={avatar}
                alt="my discord profile picture, imagine a cute cat!"
                {...props}
            />
        </PerspectiveHover>
    );
}

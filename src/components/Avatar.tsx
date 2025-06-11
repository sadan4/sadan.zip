import avatar from "@/assets/avatar.webp";

import type { ComponentProps } from "react";

export default function Avatar(props: ComponentProps<"img">) {
    return (
        <img
            className="rounded-full max-w-sm max-h-max w-auto h-auto"
            src={avatar}
            alt="my discord profile picture, imagine a cute cat!"
            {...props}
        />
    );
}

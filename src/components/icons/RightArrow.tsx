import LeftArrow from "./LeftArrow";

import type { ComponentProps } from "react";

export default function RightArrow(props: ComponentProps<"svg">) {
    return (
        <LeftArrow
            {...props}
            style={{
                ...props.style,
                transform: "scaleX(-1)",
            }}
        />
    );
}

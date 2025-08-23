import type { ComponentProps, FC } from "react";

export const ChevronDown: FC<ComponentProps<"svg">> = ({ width = 24, height = 24, ...props }) => (
    <svg
        viewBox="-2.4 -2.4 28.8 28.8"
        width={width}
        height={height}
        {...props}
    >
        <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="2"
            d="m6 9 6 6 6-6"
        />
    </svg>
);

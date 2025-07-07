import type { ComponentProps } from "react";

export default function LinkIcon(props: ComponentProps<"svg">) {
    return (
        <svg
            {...props}
            style={{
                ...props.style,
                fill: "none",
            }}
            viewBox="0 0 24 24"
        >
            <path
                d="M14 7H16C18.7614 7 21 9.23858 21 12C21 14.7614 18.7614 17 16 17H14M10 7H8C5.23858 7 3 9.23858 3 12C3 14.7614 5.23858 17 8 17H10M8 12H16"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
            />
        </svg>
    );
}

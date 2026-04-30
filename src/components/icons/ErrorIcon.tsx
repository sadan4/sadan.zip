import type { ComponentProps } from "react";

export function ErrorIcon({ width = 24, height = 24, ...props }: ComponentProps<"svg">) {
    return (
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 36 36"
            width={width}
            height={height}
            {...props}
        >
            <g>
                <path
                    d="M18 2.1a16 16 0 1 0 16 16 16 16 0 0 0-16-16m-1.4 6.7a1.4 1.4 0 0 1 2.8 0v12a1.4 1.4 0 0 1-2.8 0ZM18 28.6a1.8 1.8 0 1 1 1.8-1.8 1.8 1.8 0 0 1-1.8 1.8"
                />
                <path
                    fillOpacity="0"
                    d="M0 0h36v36H0z"
                />
            </g>
        </svg>
    );
}

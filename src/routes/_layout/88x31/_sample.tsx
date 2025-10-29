import type { ComponentProps } from "react";

export interface Sadan88x31ButtonProps extends Omit<ComponentProps<"a">, "href" | "rel"> {
}

export function Sadan88x31Button({ style, ...props }: Sadan88x31ButtonProps) {
    return (
        <a
            {...props}
            style={{
                ...style,
                width: "88px",
                height: "31px",
            }}
            href="https://www.sadan.zip"
            rel="nofollow"
        >
            <img
                src="https://www.sadan.zip/assets/88x31.png"
                style={{
                    imageRendering: "pixelated",
                    width: "88px",
                    height: "31px",
                }}
            />
        </a>
    );
}
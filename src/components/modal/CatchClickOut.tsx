import type { PropsWithChildren } from "react";

export default function CatchClickOut({ children }: PropsWithChildren) {
    return (
        <div
            onClick={(ev) => {
                ev.stopPropagation();
            }}
        >
            {children}
        </div>
    );
}

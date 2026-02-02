import type { MouseEvent, PropsWithChildren, ReactNode } from "react";

export interface TreeAccordionProps extends PropsWithChildren {
    contents: ReactNode | (() => ReactNode);
    initialShow?: boolean;
    open: boolean;
    onArrowClick?(e: MouseEvent<SVGSVGElement>): void;
}

export function TreeAccordion({ contents, children, open, onArrowClick }: TreeAccordionProps) {
    return (
        <div>
            <div className="flex">
                <svg
                    viewBox="-2.4 -2.4 28.8 28.8"
                    className="size-5 fill-none stroke-fg-500"
                    style={{
                        transform: `rotate(${open ? 0 : -90}deg)`,
                    }}
                    onClick={(e) => {
                        onArrowClick?.(e);
                    }}
                >
                    <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth="2"
                        d="m6 9 6 6 6-6"
                    />
                </svg>
                {children}
            </div>
            <div>
                {open && (typeof contents === "function" ? contents() : contents)}
            </div>
        </div>
    );
}

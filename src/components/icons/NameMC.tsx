import type { ComponentProps } from "react";


export default function NameMC(props: ComponentProps<"svg">) {
    return (
        <svg
            {...props}
            viewBox="-8 -8 16 16"
            shape-rendering="crispEdges"
        >
            <path
                d="M -8,-5.8666667 V 5.8666667 A 2.1333333,2.1333333 45 0 0 -5.8666667,8 H 5.8666667 A 2.1333333,2.1333333 135 0 0 8,5.8666667 V -5.8666667 A 2.1333333,2.1333333 45 0 0 5.8666667,-8 H -5.8666667 A 2.1333333,2.1333333 135 0 0 -8,-5.8666667 Z M -5,-5 h 8 v 2 H 5 V 5 H 3 v -8 h -6 v 8 h -2 z"
                display="inline"
                fill="currentColor"
                style={{
                    strokeWidth: 0.533333,
                }}
            />
        </svg>
    );
}

import React, { type CSSProperties, type FC, type ReactNode } from "react";

// Simple Circle component
export const Circle: FC<{ size?: number;
    color?: string; }> = ({
    size = 50,
    color = "skyblue",
}) => (
    <div
        style={{
            width: size,
            height: size,
            borderRadius: "50%",
            background: color,
            display: "inline-block",
        }}
    />
);

// Simple Square component
export const Square: React.FC<{ size?: number;
    color?: string;
    style?: CSSProperties; }> = ({
    size = 50,
    color = "salmon",
    style = {},
}) => (
    <div
        style={{
            width: size,
            height: size,
            background: color,
            display: "inline-block",
            ...style,
        }}
    />
);

// Simple Text component
export const Text: FC<{ children: ReactNode;
    color?: string; }> = ({
    children,
    color = "black",
}) => <span style={{ color }}>{children}</span>;

// Simple Button component
export const Button: React.FC<
    React.ButtonHTMLAttributes<HTMLButtonElement> & { label: string; }
> = ({ label, ...props }) => (
    <button
        style={{
            padding: "8px 16px",
            background: "#007bff",
            color: "white",
            border: "none",
            borderRadius: 4,
            cursor: "pointer",
        }}
        {...props}
    >
        {label}
    </button>
);

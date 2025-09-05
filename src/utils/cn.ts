import classNames from "classnames";

export const cn = classNames;

export default cn;

export const resizeClasses = {
    none: "resize-none",
    x: "resize-x",
    y: "resize-y",
    both: "resize",
};

export interface ResizeProp {
    resize?: keyof typeof resizeClasses;
}

export const buttonColors = {
    primary: "bg-primary active:bg-primary/65 hover:bg-primary/80 disabled:bg-primary/80",
    secondary: "bg-secondary active:bg-secondary/65 hover:bg-secondary/80 disabled:bg-secondary/80",
    accent: "bg-accent active:bg-accent/65 hover:bg-accent/80 disabled:bg-accent/80",
    neutral: "bg-neutral active:bg-neutral/87 hover:bg-neutral/93 disabled:bg-neutral/93",
    "neutral-content": "bg-neutral-content active:bg-neutral-content/65 hover:bg-neutral-content/80 disabled:bg-neutral-content/80",
    info: "bg-info active:bg-info/65 hover:bg-info/80 disabled:bg-info/80",
    "info-700": "bg-info-700 active:bg-info-700/65 hover:bg-info-700/80 disabled:bg-info-700/80",
    success: "bg-success active:bg-success/65 hover:bg-success/80 disabled:bg-success/80",
    warning: "bg-warning active:bg-warning/65 hover:bg-warning/80 disabled:bg-warning/80",
    error: "bg-error active:bg-error/65 hover:bg-error/80 disabled:bg-error/80",
} as const;

export const bgColors = {
    black: "bg-bg-100",
    "black-200": "bg-bg-200",
    "black-300": "bg-bg-300",
    white: "bg-bg-fg",
    "white-600": "bg-bg-fg-600",
    "white-700": "bg-bg-fg-700",
    "white-800": "bg-bg-fg-800",
    "white-900": "bg-bg-fg-900",
    primary: "bg-primary",
    secondary: "bg-secondary",
    accent: "bg-accent",
    neutral: "bg-neutral",
    "neutral-content": "bg-neutral-content",
    info: "bg-info",
    "info-600": "bg-info-600",
    "info-700": "bg-info-700",
    success: "bg-success",
    warning: "bg-warning",
    error: "bg-error",
} as const;


export const buttonTextColors: Record<keyof typeof buttonColors, keyof typeof textColors> = {
    primary: "black",
    secondary: "black",
    accent: "black",
    neutral: "neutral-content",
    "neutral-content": "neutral",
    info: "black",
    "info-700": "white",
    success: "black",
    warning: "black",
    error: "black",
} as const;

export const textSize = {
    xs: "text-xs",
    sm: "text-sm",
    md: "text-base",
    lg: "text-lg",
    xl: "text-xl",
    "2xl": "text-2xl",
    "3xl": "text-3xl",
    "4xl": "text-4xl",
    "5xl": "text-5xl",
    "6xl": "text-6xl",
    "7xl": "text-7xl",
    "8xl": "text-8xl",
    "9xl": "text-9xl",
};

export interface SizeProp {
    size?: keyof typeof textSize;
}

export const textWeight = {
    thin: "font-thin",
    extraLight: "font-extralight",
    light: "font-light",
    normal: "font-normal",
    medium: "font-medium",
    semiBold: "font-semibold",
    bold: "font-bold",
    extraBold: "font-extrabold",
    black: "font-black",
};

export interface WeightProp {
    weight?: keyof typeof textWeight;
}

export const textColors = {
    black: "text-bg-100",
    "black-200": "text-bg-200",
    "black-300": "text-bg-300",
    white: "text-bg-fg",
    "white-600": "text-bg-fg-600",
    "white-700": "text-bg-fg-700",
    "white-800": "text-bg-fg-800",
    "white-900": "text-bg-fg-900",
    primary: "text-primary",
    secondary: "text-secondary",
    accent: "text-accent",
    neutral: "text-neutral",
    "neutral-content": "text-neutral-content",
    info: "text-info",
    "info-600": "text-info-600",
    "info-700": "text-info-700",
    success: "text-success",
    warning: "text-warning",
    error: "text-error",
};

export interface ColorProp {
    color?: keyof typeof textColors;
}

export interface TextStyleProps extends SizeProp, WeightProp, ColorProp {

}

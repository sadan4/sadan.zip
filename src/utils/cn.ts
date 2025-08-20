import classNames from "classnames";

export const cn = classNames;

export default cn;

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

export const colors = {
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
    info: "text-info",
    "info-600": "text-info-600",
    "info-700": "text-info-700",
    success: "text-success",
    warning: "text-warning",
    error: "text-error",
};

export interface ColorProp {
    color?: keyof typeof colors;
}

export interface TextStyleProps extends SizeProp, WeightProp, ColorProp {

}

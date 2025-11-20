export enum ClickableArea {
    ARROW = 1 << 0,
    ROW = 1 << 1,
    ALL = (ROW << 1) - 1,
}

export enum ArrowPosition {
    LEFT,
    RIGHT,
}

export enum AccordionAnimation {
    NONE = 0,
    ARROW = 1 << 0,
    CONTENT = 1 << 1,
    ALL = (CONTENT << 1) - 1,
}

export enum ClickableArea {
    ARROW = 1 << 0,
    ROW = 1 << 1,
    ALL = (ROW << 1) - 1,
}

export enum ArrowPosition {
    LEFT,
    RIGHT,
}

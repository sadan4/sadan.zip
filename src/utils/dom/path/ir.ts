export enum PathCmd {
    MOVE_ABS = "M",
    MOVE_REL = "m",
    LINE_ABS = "L",
    LINE_REL = "l",
    H_LINE_ABS = "H",
    H_LINE_REL = "h",
    V_LINE_ABS = "V",
    V_LINE_REL = "v",
    CUBIC_ABS = "C",
    CUBIC_REL = "c",
    CUBIC_SMOOTH_ABS = "S",
    CUBIC_SMOOTH_REL = "s",
    QUAD_ABS = "Q",
    QUAD_REL = "q",
    QUAD_SMOOTH_ABS = "T",
    QUAD_SMOOTH_REL = "t",
    ARC_ABS = "A",
    ARC_REL = "a",
    CLOSE_PATH = "Z",
}

export type PathNode = [PathCmd, ...number[]];

/**
 * Move the _current point_ to the coordinate {@link x},{@link y}.
 */
export function moveAbs(x: number, y: number): PathNode {
    return [PathCmd.MOVE_ABS, x, y];
}
/**
 * Move the _current point_ by shifting the last known position
 * of the path by {@link dx} along the x-axis and by {@link dy} along the y-axis.
 */
export function moveRel(dx: number, dy: number): PathNode {
    return [PathCmd.MOVE_REL, dx, dy];
}

/**
 * Draw a line from the _current point_ to the end point specified by {@link x},{@link y}.
 */
export function lineAbs(x: number, y: number): PathNode {
    return [PathCmd.LINE_ABS, x, y];
}

/**
 * Draw a line from the _current point_ to the end point,
 * which is the current point shifted by {@link dx} along the x-axis and {@link dy} along the y-axis.
 */
export function lineRel(dx: number, dy: number): PathNode {
    return [PathCmd.LINE_REL, dx, dy];
}
/**
 * Draw a horizontal line from the _current point_ to the end point,
 * which is specified by the {@link x} parameter and the current point's y coordinate.
 */
export function hLineAbs(x: number): PathNode {
    return [PathCmd.H_LINE_ABS, x];
}

/**
 * Draw a horizontal line from the _current point_ to the end point,
 * which is specified by the current point shifted by {@link dx} along the x-axis and the current point's y coordinate.
 */
export function hLineRel(dx: number): PathNode {
    return [PathCmd.H_LINE_REL, dx];
}

/**
 * Draw a vertical line from the _current point_ to the end point,
 * which is specified by the {@link y} parameter and the current point's x coordinate.
 */
export function vLineAbs(y: number): PathNode {
    return [PathCmd.V_LINE_ABS, y];
}

/**
 * Draw a vertical line from the _current point_ to the end point,
 * which is specified by the current point shifted by {@link dy} along the y-axis and the current point's x coordinate.
 */
export function vLineRel(dy: number): PathNode {
    return [PathCmd.V_LINE_REL, dy];
}

/**
 * Draw a cubic Bézier curve from the _current point_ to the _end point_ specified by {@link x},{@link y}.
 * The _start control point_ is specified by {@link x1},{@link y1} and the _end control point_ is specified by {@link x2},{@link y2}.
 *
 * @param x1 start control point x coordinate
 * @param y1 start control point y coordinate
 * @param x2 end control point x coordinate
 * @param y2 end control point y coordinate
 * @param x end point x coordinate
 * @param y end point y coordinate
 */
export function cubicAbs(
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    x: number,
    y: number,
): PathNode {
    return [PathCmd.CUBIC_ABS, x1, y1, x2, y2, x, y];
}

/**
 * Draw a cubic Bézier curve from the _current point_ to the _end point_,
 * which is the current point shifted by {@link dx} along the x-axis and {@link dy} along the y-axis.
 * The start control point is the current point (starting point of the curve)
 * shifted by {@link dx1} along the x-axis and {@link dy1} along the y-axis.
 * The end control point is the current point (starting point of the curve)
 * shifted by {@link dx2} along the x-axis and {@link dy2} along the y-axis. 
 */
export function cubicRel(
    dx1: number,
    dy1: number,
    dx2: number,
    dy2: number,
    dx: number,
    dy: number,
): PathNode {
    return [PathCmd.CUBIC_REL, dx1, dy1, dx2, dy2, dx, dy];
}

/**
 * Draw a smooth cubic Bézier curve from the current point to the end point specified by {@link x},{@link y}.
 * The end control point is specified by {@link x2},{@link y2}.
 * The start control point is the reflection of the end control point of the previous curve command about the current point.
 * If the previous command wasn't a cubic Bézier curve,
 * the start control point is the same as the curve starting point (current point).
 */
export function cubicSmoothAbs(
    x2: number,
    y2: number,
    x: number,
    y: number,
): PathNode {
    return [PathCmd.CUBIC_SMOOTH_ABS, x2, y2, x, y];
}

/**
 * Draw a smooth cubic Bézier curve from the current point to the end point,
 * which is the current point shifted by {@link dx} along the x-axis and {@link dy} along the y-axis.
 * The end control point is the current point (starting point of the curve) shifted by {@link dx2} along the x-axis and {@link dy2} along the y-axis.
 * The start control point is the reflection of the end control point of the previous curve command about the current point.
 * If the previous command wasn't a cubic Bézier curve,
 * the start control point is the same as the curve starting point (current point).
 */
export function cubicSmoothRel(
    dx2: number,
    dy2: number,
    dx: number,
    dy: number,
): PathNode {
    return [PathCmd.CUBIC_SMOOTH_REL, dx2, dy2, dx, dy];
}

/**
 * Draw a quadratic Bézier curve from the current point to the end point specified by {@link x},{@link y}.
 * The control point is specified by {@link x1},{@link y1}.
 */
export function quadAbs(
    x1: number,
    y1: number,
    x: number,
    y: number,
): PathNode {
    return [PathCmd.QUAD_ABS, x1, y1, x, y];
}

/**
 * Draw a quadratic Bézier curve from the current point to the end point,
 * which is the current point shifted by {@link dx} along the x-axis and {@link dy} along the y-axis.
 * The control point is the current point (starting point of the curve) shifted by {@link dx1} along the x-axis and {@link dy1} along the y-axis.
 */
export function quadRel(
    dx1: number,
    dy1: number,
    dx: number,
    dy: number,
): PathNode {
    return [PathCmd.QUAD_REL, dx1, dy1, dx, dy];
}

/**
 * Draw a smooth quadratic Bézier curve from the current point to the end point specified by {@link x},{@link y}.
 * The control point is the reflection of the control point of the previous curve command about the current point.
 * If the previous command wasn't a quadratic Bézier curve,
 * the control point is the same as the curve starting point (current point).
 */
export function quadSmoothAbs(
    x: number,
    y: number,
): PathNode {
    return [PathCmd.QUAD_SMOOTH_ABS, x, y];
}

/**
 * Draw a smooth quadratic Bézier curve from the current point to the end point,
 * which is the current point shifted by {@link dx} along the x-axis and {@link dy} along the y-axis.
 * The control point is the reflection of the control point of the previous curve command about the current point.
 * If the previous command wasn't a quadratic Bézier curve,
 * the control point is the same as the curve starting point (current point).
 */
export function quadSmoothRel(
    dx: number,
    dy: number,
): PathNode {
    return [PathCmd.QUAD_SMOOTH_REL, dx, dy];
}
/**
 * Draw an Arc curve from the current point to the coordinate {@link x},{@link y}.
 *
 *  The center of the ellipse used to draw the arc is determined automatically based on the other parameters of the command:
 * 
 * - {@link rx} and {@link ry} are the two radii of the ellipse;
 * - {@link angle} represents a rotation (in degrees) of the ellipse relative to the x-axis;
 * - {@link largeArcFlag} and {@link sweepFlag} allow to choose which arc must be drawn as 4 possible arcs can be drawn out of the other parameters.
 *   - {@link largeArcFlag} allows to choose one of the large arc (`true`) or small arc (`false`),
 *   - {@link sweepFlag} allows to choose one of the clockwise turning arc (`true`) or counterclockwise turning arc (`false`)
 * 
 * The coordinate {@link x},{@link y} becomes the new current point for the next command.
 */
export function arcAbs(
    rx: number,
    ry: number,
    angle: number,
    largeArcFlag: boolean,
    sweepFlag: boolean,
    x: number,
    y: number,
): PathNode {
    return [PathCmd.ARC_ABS, rx, ry, angle, +largeArcFlag, +sweepFlag, x, y];
}

/**
 * Draw an Arc curve from the current point to a point for which coordinates are those of the current point shifted by dx along the x-axis and dy along the y-axis.
 *
 * The center of the ellipse used to draw the arc is determined automatically based on the other parameters of the command:
 *
 * - {@link rx} and {@link ry} are the two radii of the ellipse;
 * - {@link angle} represents a rotation (in degrees) of the ellipse relative to the x-axis;
 * - {@link largeArcFlag} and {@link sweepFlag} allow to choose which arc must be drawn as 4 possible arcs can be drawn out of the other parameters.
 *   - {@link largeArcFlag} allows to choose one of the large arc (`true`) or small arc (`false`),
 *   - {@link sweepFlag} allows to choose one of the clockwise turning arc (`true`) or counterclockwise turning arc (`false`)
 * 
 * The current point gets its X and Y coordinates shifted by {@link dx} and {@link dy} for the next command.
 */
export function arcRel(
    rx: number,
    ry: number,
    angle: number,
    largeArcFlag: boolean,
    sweepFlag: boolean,
    dx: number,
    dy: number,
): PathNode {
    return [PathCmd.ARC_REL, rx, ry, angle, +largeArcFlag, +sweepFlag, dx, dy];
}

export function closePath(): PathNode {
    return [PathCmd.CLOSE_PATH];
}

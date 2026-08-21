import {
    arcAbs,
    cubicAbs,
    cubicSmoothAbs,
    hLineAbs,
    lineAbs,
    moveAbs,
    PathCmd,
    type PathNode,
    quadAbs,
    quadSmoothAbs,
    vLineAbs,
} from "./ir";

/**
 * Shift every node of {@link path} by {@link dx} along the x-axis and {@link dy} along the y-axis.
 */
export function offsetPath(path: PathNode[], dx: number, dy: number): PathNode[] {
    return path.map((node) => offsetNode(node, dx, dy));
}

/**
 * Shift a single {@link node} by {@link dx} along the x-axis and {@link dy} along the y-axis.
 *
 * Relative commands describe an offset from the _current point_ rather than a coordinate,
 * so they are translation invariant and returned unchanged.
 * The same goes for the radii, the rotation and the flags of an arc command.
 */
function offsetNode(node: PathNode, dx: number, dy: number): PathNode {
    switch (node[0]) {
        case PathCmd.MOVE_ABS: {
            const [, x, y] = node;

            return moveAbs(x + dx, y + dy);
        }
        case PathCmd.LINE_ABS: {
            const [, x, y] = node;

            return lineAbs(x + dx, y + dy);
        }
        case PathCmd.H_LINE_ABS: {
            const [, x] = node;

            return hLineAbs(x + dx);
        }
        case PathCmd.V_LINE_ABS: {
            const [, y] = node;

            return vLineAbs(y + dy);
        }
        case PathCmd.CUBIC_ABS: {
            const [, x1, y1, x2, y2, x, y] = node;

            return cubicAbs(x1 + dx, y1 + dy, x2 + dx, y2 + dy, x + dx, y + dy);
        }
        case PathCmd.CUBIC_SMOOTH_ABS: {
            const [, x2, y2, x, y] = node;

            return cubicSmoothAbs(x2 + dx, y2 + dy, x + dx, y + dy);
        }
        case PathCmd.QUAD_ABS: {
            const [, x1, y1, x, y] = node;

            return quadAbs(x1 + dx, y1 + dy, x + dx, y + dy);
        }
        case PathCmd.QUAD_SMOOTH_ABS: {
            const [, x, y] = node;

            return quadSmoothAbs(x + dx, y + dy);
        }
        case PathCmd.ARC_ABS: {
            const [, rx, ry, angle, largeArcFlag, sweepFlag, x, y] = node;

            return arcAbs(rx, ry, angle, !!largeArcFlag, !!sweepFlag, x + dx, y + dy);
        }
        case PathCmd.MOVE_REL:
        case PathCmd.LINE_REL:
        case PathCmd.H_LINE_REL:
        case PathCmd.V_LINE_REL:
        case PathCmd.CUBIC_REL:
        case PathCmd.CUBIC_SMOOTH_REL:
        case PathCmd.QUAD_REL:
        case PathCmd.QUAD_SMOOTH_REL:
        case PathCmd.ARC_REL:
        case PathCmd.CLOSE_PATH: {
            return node;
        }
    }
}

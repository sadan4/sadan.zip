export const SYM_NOT_COMPUTED = Symbol("PropViewer.NOT_COMPUTED");

export enum PropViewerFlags {
    NONE = 0,
    EAGER_GETTERS = 1 << 0,
    CAN_RECALCULATE_GETTERS = 1 << 1,
    SHOW_SETTERS = 1 << 3,
}

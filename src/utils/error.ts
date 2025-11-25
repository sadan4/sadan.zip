export function error(message?: string): never {
    throw new Error(message);
}

class AssertionError extends Error {
    name = "AssertionError";

    constructor(msg?: string) {
        super(msg);
    }
}

class NotImplementedError extends Error {
    name = "NotImplementedError";

    constructor(msg?: string) {
        super(msg);
    }
}

export function assert(cond: unknown, msg?: string): asserts cond {
    if (!cond) {
        throw new AssertionError(msg);
    }
}

export function unreachable(msg?: string): never {
    throw new AssertionError(msg || "unreachable");
}

export function todo(msg?: string) {
    throw new NotImplementedError(msg);
}

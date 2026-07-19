export class AssertionError extends Error {
    name = "AssertionError";

    constructor(msg?: string) {
        super(msg);
    }
}

class DebugAssertionError extends AssertionError {
    name = "DebugAssertionError";

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

class UnavailableImportError extends Error {
    name = "UnavailableImportError";

    constructor(msg?: string) {
        super(msg);
    }
}

/**
 * An assertion with an expression that is always falsy will always fail.
 * 
 * NOTE: NaN is falsy, but not included because there is no literal type for it
 * 
 * @throws an {@link AssertionError} always
 * 
 * @see {@link unreachable} and {@link error} for better uses if you are passing a literal
 * @see {@link https://developer.mozilla.org/en-US/docs/Glossary/Falsy|MDN - Falsy}
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/HTMLAllCollection|MDN - HTMLAllCollection}
 */
export function assert(cond: null | undefined | false | 0 | 0n | "" | HTMLAllCollection, msg?: string): never;
/**
 * Assert {@link cond} is truthy
 * 
 * @throws an {@link AssertionError} if {@link cond} is falsy
 */
export function assert(cond: unknown, msg?: string): asserts cond;
export function assert(cond: unknown, msg?: string): asserts cond {
    if (!cond) {
        const err = new AssertionError(msg);

        AssertionError.captureStackTrace(err, assert);
        throw err;
    }
}

// debug_assert is snake case to make it stand out from other identifiers

/**
 * An assertion with an expression that is always falsy will always fail.
 * 
 * NOTE: NaN is falsy, but not included because there is no literal type for it
 * 
 * @throws an {@link DebugAssertionError} always
 * 
 * @deprecated You should never call this function with a falsy literal.
 * 
 * @see {@link unreachable} and {@link error} for better uses if you are passing a literal
 * @see {@link assert} for runtime assertions
 * @see {@link https://developer.mozilla.org/en-US/docs/Glossary/Falsy|MDN - Falsy}
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/HTMLAllCollection|MDN - HTMLAllCollection}
 */
export function debug_assert(cond: null | undefined | false | 0 | 0n | "" | HTMLAllCollection, msg?: string): never;
/**
 * Assert {@link cond} is truthy
 * 
 * Only runs in development mode `import.meta.env.DEV`
 * 
 * @throws an {@link DebugAssertionError} if {@link cond} is falsy
 * 
 * @see {@link assert} for runtime assertions
 */
export function debug_assert(cond: unknown, msg?: string): asserts cond;
export function debug_assert(cond: unknown, msg?: string): asserts cond {
    if (import.meta.env.DEV) {
        if (!cond) {
            const err = new DebugAssertionError(msg);

            // oxlint-disable-next-line typescript/no-deprecated -- typescript is too smart for our own good
            DebugAssertionError.captureStackTrace(err, debug_assert);
            throw err;
        }
    }
}

export function unreachable(msg?: string): never {
    const err = new AssertionError(msg || "unreachable");

    AssertionError.captureStackTrace(err, unreachable);
    throw err;
}

export function error(message?: string): never {
    const err = new Error(message);

    Error.captureStackTrace(err, error);
    throw err;
}


export function todo(msg?: string): never {
    const err = new NotImplementedError(msg);

    NotImplementedError.captureStackTrace(err, todo);
    throw err;
}

export function unavailableImport<T = never>(importName?: string): T {
    // oxlint-disable-next-line unicorn/consistent-function-scoping -- we want a new instance every time
    function func() { }

    Object.defineProperty(func, "name", {
        configurable: true,
        writable: false,
        enumerable: false,
        value: `${importName || "import"}Proxy`,
    });

    return new Proxy(func, {
        get() {
            const err = new UnavailableImportError(`${importName || "This import"} is unavailable in the current environment.`);

            UnavailableImportError.captureStackTrace(err, unavailableImport);
            throw err;
        },
        apply() {
            const err = new UnavailableImportError(`${importName || "This import"} is unavailable in the current environment.`);

            UnavailableImportError.captureStackTrace(err, unavailableImport);
            throw err;
        },
        construct() {
            const err = new UnavailableImportError(`${importName || "This import"} is unavailable in the current environment.`);

            UnavailableImportError.captureStackTrace(err, unavailableImport);
            throw err;
        },
    }) as T;
}

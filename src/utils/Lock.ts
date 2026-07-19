import { assert } from "./error";

export class Lock {
    private _locked: boolean;

    constructor(initialState = false) {
        this._locked = initialState;
    }

    get locked(): boolean {
        return this._locked;
    }

    lock(): void {
        this._locked = true;
    }

    unlock(): void {
        this._locked = false;
    }

    runIf<T extends {} | null>(fn: () => T): T | undefined {
        return this._locked ? undefined : fn();
    }


    bindIf<R extends {} | null, A extends any[]>(fn: (...args: A) => R): ((...args: A) => R | undefined);
    bindIf<F extends (...args: any[]) => void>(fn: F): F;
    bindIf<R extends {} | null, A extends any[]>(fn: (...args: A) => R): ((...args: A) => R | undefined) {
        return (...args) => {
            return this._locked ? undefined : fn(...args);
        };
    }

    /**
     * unlocked when the promise is resolved
     */
    lockWhile<T extends Promise<unknown>>(fn: () => T): T;
    /**
     * unlocked when the function returns
     */
    lockWhile<T>(fn: () => T): T;
    lockWhile<T>(fn: () => T): T {
        assert(!this._locked, "cannot call lockWhile while already locked");

        let promise = false;

        try {
            this._locked = true;

            const ret = fn();

            if (ret instanceof Promise) {
                promise = true;
                ret.finally(() => {
                    this._locked = false;
                });
            }

            return ret;
        } finally {
            if (!promise) {
                this._locked = false;
            }
        }
    }
}

import { error } from "@/utils/error";

import type { CheckedInputProps, LenCheck } from ".";

function assertValidLenCheck(lenCheck: LenCheck) {
    // both undefined
    if (!(lenCheck.min || lenCheck.max)) {
        error("Invalid length check");
    }
    // min < 0
    if (lenCheck.min != null && lenCheck.min < 0) {
        error("Invalid minimum length");
    }
    // max <= 0
    if (lenCheck.max != null && lenCheck.max <= 0) {
        error("Invalid maximum length");
    }
    // min >= max
    if (lenCheck.min != null && lenCheck.max != null && lenCheck.min >= lenCheck.max) {
        error("Invalid length check");
    }
}

export function validateLength(check: LenCheck, value: string): boolean {
    assertValidLenCheck(check);

    const len = value.length;

    if (check.min != null && len < check.min) {
        return false;
    }
    if (check.max != null && len > check.max) {
        return false;
    }
    return true;
}

export function validateCheckedInput(msg: string, check: CheckedInputProps["check"]): boolean {
    if (typeof check === "function") {
        return check(msg);
    } else if (check instanceof RegExp) {
        return check.test(msg);
    } else if (check.type === "len") {
        return validateLength(check, msg);
    }
    throw new Error("invalid check type");
}

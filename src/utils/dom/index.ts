import { type DisposableString, disposableString } from "../string";

export function isMobileDevice(): boolean {
    // i guess people with laptops are fucked (myself included)
    return navigator.maxTouchPoints > 0;
}

/**
 * Gives the default value for an <input type="range" /> element when the default value is not provided
 */
export function makeDefaultForInputRange(min = 0, max = 100) {
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/range#value
    return max < min
        ? min
        : min + ((max - min) / 2);
}

/**
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons|MDN}
 */
export const enum MouseButtons {
    /**
     * No button or un-initialized
     */
    NONE = 0,
    /**
     * Primary button (usually the left button)
     */
    PRIMARY = 1,
    /**
     * Secondary button (usually the right button)
     */
    SECONDARY = 2,
    /**
     * Auxiliary button (usually the mouse wheel button or middle button)
     */
    AUXILIARY = 4,
    /**
     * 4th button (typically the "Browser Back" button)
     */
    BACK = 8,
    /**
     * 5th button (typically the "Browser Forward" button)
     */
    FORWARD = 16,
}

export function withObjectURL(obj: Blob | MediaSource): DisposableString {
    return disposableString(URL.createObjectURL(obj), URL.revokeObjectURL);
}

export function download(file: File) {
    using url = withObjectURL(file);
    downloadUrl(url, file.name);
}

export function downloadUrl(url: string, fileName: string) {
    const a = document.createElement("a");

    a.href = url;

    if (fileName) {
        a.download = fileName;
    }

    a.click();
}

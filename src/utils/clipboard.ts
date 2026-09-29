import { type IToastStore, ToastPosition, ToastType } from "@/stores/ToastStore";

export function copy(text: string): Promise<void> {
    return navigator.clipboard.writeText(text);
}
export function paste(): Promise<string> {
    return navigator.clipboard.readText();
}

export async function copyWithNotify(text: string, toaster: IToastStore): Promise<void> {
    try {
        await copy(text);
        toaster.pushToast({
            id: toaster.genId(),
            duration: 1500,
            type: ToastType.SUCCESS,
            pos: ToastPosition.TOP,
            render() {
                return "Copied!";
            },
        });
    } catch (e) {
        console.error(e);
        toaster.pushToast({
            id: toaster.genId(),
            duration: 1500,
            type: ToastType.ERROR,
            pos: ToastPosition.TOP,
            render() {
                return "Failed to copy! Check console.";
            },
        });
    }
}

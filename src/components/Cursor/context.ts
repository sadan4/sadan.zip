import { namedContext } from "@/utils/devtools";
import { isMobileDevice } from "@/utils/dom";

export const CustomCursorContext = namedContext<boolean | undefined>(isMobileDevice() || undefined, "CustomCursorContext");

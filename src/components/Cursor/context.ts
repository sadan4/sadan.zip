import { namedContext } from "@/utils/devtools";
import { isMobileDevice } from "@/utils/dom";

export const CustomCursorContext = namedContext<boolean>(isMobileDevice(), "CustomCursorContext");

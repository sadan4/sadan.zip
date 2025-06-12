import cn from "@/utils/cn";


export interface DotCursorProps {
    className?: string;
    radius?: number;
}

export function DotCursor({ className, radius = 6 }: DotCursorProps) {
    return (
        <div
            className={cn("rounded-full", className)}
            style={{
                width: `${radius}px`,
                height: `${radius}px`,
            }}
        />
    );
}


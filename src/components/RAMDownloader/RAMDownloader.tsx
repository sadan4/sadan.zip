import { useState, useEffect } from "react";
import { Button } from "../Button";
import { Text } from "../Text";
import { Box } from "../layout/Box";
import * as styles from "./RAMDownloader.module.scss";
import { HorizontalLine } from "../Lines";
import { cn } from "@/utils/cn";

const RAM_OPTIONS = [
    { label: "4 GB", value: 4 },
    { label: "8 GB", value: 8 },
    { label: "16 GB", value: 16 },
    { label: "32 GB", value: 32 },
    { label: "64 GB", value: 64 },
    { label: "128 GB", value: 128 },
    { label: "1 TB (EXTREME)", value: 1024 },
];

export function RAMDownloader() {
    const [selectedRam, setSelectedRam] = useState<number | null>(null);
    const [progress, setProgress] = useState(0);
    const [status, setStatus] = useState<"idle" | "downloading" | "complete">("idle");
    const [speed, setSpeed] = useState(0);

    useEffect(() => {
        let interval: NodeJS.Timeout;
        if (status === "downloading" && selectedRam !== null) {
            interval = setInterval(() => {
                setProgress((prev) => {
                    if (prev >= 100) {
                        setStatus("complete");
                        return 100;
                    }
                    // Random progress increments
                    const increment = Math.random() * 2;
                    
                    // Speed scales with RAM size: ~12.5 MB/s per GB
                    // 4 GB -> ~50 MB/s
                    // 1 TB -> ~12.8 GB/s
                    const baseSpeed = selectedRam * 12.5;
                    const jitter = Math.random() * baseSpeed * 0.3;
                    setSpeed(baseSpeed + jitter);
                    
                    return Math.min(prev + increment, 100);
                });
            }, 100);
        }
        return () => clearInterval(interval);
    }, [status, selectedRam]);

    const handleStart = (val: number) => {
        setSelectedRam(val);
        setStatus("downloading");
        setProgress(0);
    };

    const handleReset = () => {
        setStatus("idle");
        setSelectedRam(null);
        setProgress(0);
    };

    return (
        <Box className="max-w-150 my-8 mx-auto p-8 text-center bg-bg-100 rounded-lg shadow-lg">
            <Text size="2xl" weight="bold" className="mb-4">
                Download More RAM
            </Text>
            <Text className="opacity-70">
                Is your computer running slow? Don't buy expensive hardware! 
                Just download more RAM directly to your system using our patented 
                cloud-based memory virtualization technology.
            </Text>
            <HorizontalLine />

            {status === "idle" && (
                <div className="flex flex-wrap gap-4 justify-evenly">
                    {RAM_OPTIONS.map((opt) => (
                        <Button
                            key={opt.value}
                            onClick={() => handleStart(opt.value)}
                            color="primary"
                            className={"w-fit"}
                        >
                            Download {opt.label}
                        </Button>
                    ))}
                </div>
            )}

            {status === "downloading" && (
                <div className="mt-8">
                    <Text size="lg" className="mb-2">
                        Downloading {selectedRam === 1024 ? "1 TB" : `${selectedRam} GB`} of RAM...
                    </Text>
                    <div className="w-full h-5 bg-bg-300 rounded-[10px] overflow-hidden mt-4">
                        <div 
                            className="h-full bg-primary-400 transition-[width] duration-100 ease-linear" 
                            style={{ width: `${progress}%` }}
                        />
                    </div>
                    <div className="flex justify-between mt-2">
                        <Text size="sm">{Math.round(progress)}%</Text>
                        <Text size="sm">{speed.toFixed(2)} MB/s</Text>
                    </div>
                </div>
            )}

            {status === "complete" && (
                <div className={cn("mt-8", styles.fadeIn)}>
                    <Text size="xl" color="success" weight="bold" className="mb-4">
                        Success!
                    </Text>
                    <Text className="mb-6">
                        {selectedRam === 1024 ? "1 TB" : `${selectedRam} GB`} of high-speed RAM has been 
                        successfully downloaded and integrated into your system BIOS.
                    </Text>
                    <Text className="mb-8 italic">
                        Note: You may need to download more storage space to house your new RAM.
                    </Text>
                    <div className="flex gap-4 justify-center">
                        <Button onClick={handleReset} colorType="outline">
                            Download More
                        </Button>
                        <Button 
                            onClick={() => window.open("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "_blank")}
                        >
                            Verify RAM
                        </Button>
                    </div>
                </div>
            )}
        </Box>
    );
}

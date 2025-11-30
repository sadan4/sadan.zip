import styles from "./styles.module.scss";

import { useEffect, useRef } from "react";

interface Snowflake {
    x: number;
    y: number;
    radius: number;
    speed: number;
    wind: number;
    opacity: number;
    reverseChance: number;
    fadeOutPoint: number;
    fadeSpeed: number;
}

export interface SnowCanvasProps {
    density?: number;
    minSpeed?: number;
    maxSpeed?: number;
    minSize?: number;
    maxSize?: number;
    windStrength?: number;
}

export function SnowCanvas({
    density = 75,
    minSpeed = 1,
    maxSpeed = 2.5,
    minSize = 2,
    maxSize = 4,
    windStrength = 0.7,
}: SnowCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const snowflakesRef = useRef<Snowflake[]>([]);
    const animationFrameRef = useRef(0);

    useEffect(() => {
        const _canvas = canvasRef.current;

        if (!_canvas) {
            return;
        }

        const canvas = _canvas;
        const _ctx = canvas.getContext("2d");

        if (!_ctx) {
            return;
        }

        const ctx = _ctx;

        const snowColor = getComputedStyle(document.documentElement)
            .getPropertyValue("--color-fg-500")
            .trim();

        function resizeCanvas() {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        }

        function createSnowflake(): Snowflake {
            return {
                x: Math.random() * canvas.width,
                y: Math.random() * canvas.height,
                radius: (Math.random() * (maxSize - minSize)) + minSize,
                speed: (Math.random() * (maxSpeed - minSpeed)) + minSpeed,
                wind: (Math.random() - 0.5) * windStrength,
                opacity: (Math.random() * 0.5) + 0.5,
                reverseChance: Math.random() * 0.002,
                fadeOutPoint: Math.random() < 0.7 ? (Math.random() * 0.6) + 0.2 : 1,
                fadeSpeed: (Math.random() * 0.01) + 0.005,
            };
        }

        function initSnowflakes() {
            const count = Math.floor((canvas.width * canvas.height) / 10000) * density / 10;

            snowflakesRef.current = Array.from({ length: count }, createSnowflake);
        }

        function updateSnowflake(snowflake: Snowflake) {
            if (Math.random() < snowflake.reverseChance) {
                snowflake.wind *= -1;
            }

            snowflake.y += snowflake.speed;
            snowflake.x += snowflake.wind;

            const progress = snowflake.y / canvas.height;

            if (progress > snowflake.fadeOutPoint) {
                snowflake.opacity -= snowflake.fadeSpeed;
            }

            if (snowflake.y > canvas.height || snowflake.opacity <= 0) {
                snowflake.y = -snowflake.radius;
                snowflake.x = Math.random() * canvas.width;
                snowflake.opacity = (Math.random() * 0.5) + 0.5;
                snowflake.fadeOutPoint = Math.random() < 0.7 ? (Math.random() * 0.6) + 0.2 : 1;
            }

            if (snowflake.y < -snowflake.radius) {
                snowflake.y = canvas.height + snowflake.radius;
                snowflake.x = Math.random() * canvas.width;
            }

            if (snowflake.x > canvas.width + snowflake.radius) {
                snowflake.x = -snowflake.radius;
            } else if (snowflake.x < -snowflake.radius) {
                snowflake.x = canvas.width + snowflake.radius;
            }
        }

        function drawSnowflake(snowflake: Snowflake) {
            ctx.beginPath();
            ctx.arc(snowflake.x, snowflake.y, snowflake.radius, 0, Math.PI * 2);
            ctx.fillStyle = snowColor;
            ctx.globalAlpha = snowflake.opacity * 0.6;
            ctx.fill();
            ctx.globalAlpha = 1;
        }

        function animate() {
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            const snowflakes = snowflakesRef.current;

            for (let i = 0; i < snowflakes.length; i++) {
                updateSnowflake(snowflakes[i]);
                drawSnowflake(snowflakes[i]);
            }

            animationFrameRef.current = requestAnimationFrame(animate);
        }

        resizeCanvas();
        initSnowflakes();
        animate();

        function handleResize() {
            resizeCanvas();
            initSnowflakes();
        }

        window.addEventListener("resize", handleResize);

        return () => {
            window.removeEventListener("resize", handleResize);
            cancelAnimationFrame(animationFrameRef.current);
        };
    }, [density, minSpeed, maxSpeed, minSize, maxSize, windStrength]);

    return (
        <canvas
            ref={canvasRef}
            className={styles.canvas}
        />
    );
}

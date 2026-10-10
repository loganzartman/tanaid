import { createEffect, createMemo } from "solid-js";
import cursorDarkDefault from "../cursor/arrow-dark.png";
import cursorLightDefault from "../cursor/arrow-light.png";
import cursorDarkPointer from "../cursor/hand-dark.png";
import cursorLightPointer from "../cursor/hand-light.png";
import cursorDarkText from "../cursor/beam-dark.png";
import cursorLightText from "../cursor/beam-light.png";
import { createPixelPerfectScale } from "./PixelPerfect";

export function applyCursorStyles() {
  const scale = createPixelPerfectScale();

  const cursorDefault = createMemo(() =>
    makeCssCursor({
      lightSrc: cursorLightDefault,
      darkSrc: cursorDarkDefault,
      globalScale: scale().globalScale,
      unitScale: scale().unitScale,
      origin: [6, 6],
      fallback: "default",
    }),
  );
  const cursorPointer = createMemo(() =>
    makeCssCursor({
      lightSrc: cursorLightPointer,
      darkSrc: cursorDarkPointer,
      globalScale: scale().globalScale,
      unitScale: scale().unitScale,
      origin: [10, 6],
      fallback: "pointer",
    }),
  );
  const cursorText = createMemo(() =>
    makeCssCursor({
      lightSrc: cursorLightText,
      darkSrc: cursorDarkText,
      globalScale: scale().globalScale,
      unitScale: scale().unitScale,
      origin: [15, 15],
      fallback: "text",
    }),
  );

  createEffect(
    () => ({
      cursorDefault: cursorDefault(),
      cursorPointer: cursorPointer(),
      cursorText: cursorText(),
    }),
    ({ cursorDefault, cursorPointer, cursorText }) => {
      document.documentElement.style.setProperty("--cursor-default", cursorDefault);
      document.documentElement.style.setProperty("--cursor-pointer", cursorPointer);
      document.documentElement.style.setProperty("--cursor-text", cursorText);
    },
  );
}

export async function makeCssCursor({
  lightSrc,
  darkSrc,
  unitScale,
  globalScale,
  origin,
  fallback,
}: {
  lightSrc: string;
  darkSrc: string;
  unitScale: number;
  globalScale: number;
  origin: [number, number];
  fallback: string;
}): Promise<string> {
  const dark = new Image();
  const light = new Image();
  dark.src = darkSrc;
  light.src = lightSrc;
  await Promise.all([
    new Promise<void>((res, rej) => {
      dark.onload = () => res();
      dark.onerror = () => rej();
    }),
    new Promise<void>((res, rej) => {
      light.onload = () => res();
      light.onerror = () => rej();
    }),
  ]);

  const darkCursor = await renderCursor({ img: dark, scale: unitScale });
  const lightCursor = await renderCursor({ img: light, scale: unitScale });
  const originX = Math.ceil(origin[0] * unitScale * globalScale);
  const originY = Math.ceil(origin[1] * unitScale * globalScale);
  return `image-set(light-dark(url('${lightCursor}'), url('${darkCursor}')) ${1 / globalScale}x) ${originX} ${originY}, ${fallback}`;
}

async function renderCursor({
  img,
  scale,
}: {
  img: HTMLImageElement;
  scale: number;
}): Promise<string> {
  const withShadow = new OffscreenCanvas(img.width, img.height);
  const cShadow = withShadow.getContext("2d");
  if (!cShadow) {
    throw new Error("Failed to create canvas context");
  }
  cShadow.filter = "drop-shadow(2.6px 1.5px 1.5px rgb(0 0 0 / 0.32))";
  cShadow.drawImage(img, 0, 0);

  const scaled = new OffscreenCanvas(Math.ceil(img.width * scale), Math.ceil(img.height * scale));
  const cScaled = scaled.getContext("2d");
  if (!cScaled) {
    throw new Error("Failed to create canvas context");
  }
  cScaled.imageSmoothingEnabled = false;
  cScaled.drawImage(withShadow, 0, 0, scaled.width, scaled.height);

  const blob = await scaled.convertToBlob();
  return URL.createObjectURL(blob);
}

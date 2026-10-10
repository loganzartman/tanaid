import cursorDarkBase from "../img/cur-dark.png";
import cursorLightBase from "../img/cur-light.png";

export async function createCssCursor({ scale }: { scale: number }): Promise<string> {
  const dark = new Image();
  const light = new Image();
  dark.src = cursorDarkBase;
  light.src = cursorLightBase;
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

  const darkCursor = await renderCursor({ img: dark, scale });
  const lightCursor = await renderCursor({ img: light, scale });
  return `image-set(light-dark(url('${lightCursor}'), url('${darkCursor}')) ${scale}x) 6 6, auto`;
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
  cShadow.filter = "drop-shadow(3px 1px 1.5px rgb(0 0 0 / 0.32))";
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

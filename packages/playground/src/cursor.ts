export async function makeCssCursor({
  lightSrc,
  darkSrc,
  unitScale,
  globalScale,
}: {
  lightSrc: string;
  darkSrc: string;
  unitScale: number;
  globalScale: number;
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
  const origin = Math.ceil(6 * unitScale * globalScale);
  return `image-set(light-dark(url('${lightCursor}'), url('${darkCursor}')) ${1 / globalScale}x) ${origin} ${origin}, auto`;
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

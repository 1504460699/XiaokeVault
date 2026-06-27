import sharp from "sharp";
import { readFileSync } from "fs";

// 把 brand/icon.svg 渲染成 1024×1024 PNG（tauri icon 的源图）
const svg = readFileSync("brand/icon.svg");
await sharp(svg, { density: 384 })
  .resize(1024, 1024)
  .png()
  .toFile("brand/icon-1024.png");
console.log("written brand/icon-1024.png");

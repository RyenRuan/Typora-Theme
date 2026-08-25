import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const wasm = await readFile(resolve("dist/renderer.wasm"));
const { instance } = await WebAssembly.instantiate(wasm, {});
const api = instance.exports;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function render(source) {
  const input = encoder.encode(source);
  const pointer = api.alloc(input.length);
  new Uint8Array(api.memory.buffer, pointer, input.length).set(input);
  api.render(pointer, input.length);
  const output = decoder.decode(
    new Uint8Array(api.memory.buffer, api.result_ptr(), api.result_len()),
  );
  const ok = api.last_ok() === 1;
  api.dealloc(pointer, input.length);
  return { ok, output };
}

const complex = `flowchart LR
  A([开始]) --> B[收集需求]
  B --> C{方案是否通过?}
  C -->|是| D[客户端开发]
  C -->|是| E[服务端开发]
  D --> F[联合测试]
  E --> F
  F --> G{质量验收?}
  G -->|通过| H([发布上线])
  G -. 回退修复 .-> D
  G -->|取消| I([结束])`;

const started = performance.now();
for (let index = 0; index < 100; index += 1) {
  const result = render(complex);
  if (!result.ok) throw new Error(result.output);
  if (!result.output.includes("data-ryen-renderer=\"0.1.0\"")) {
    throw new Error("Renderer marker missing");
  }
  if (!result.output.includes("方案是否通过?")) throw new Error("Chinese label missing");
}
const elapsed = performance.now() - started;

const fallback = render("sequenceDiagram\nA->>B: hello");
if (fallback.ok) throw new Error("Unsupported diagram should fall back to native Mermaid");

console.log(
  JSON.stringify({
    renders: 100,
    totalMs: Number(elapsed.toFixed(2)),
    averageMs: Number((elapsed / 100).toFixed(3)),
    fallback: fallback.output,
  }),
);


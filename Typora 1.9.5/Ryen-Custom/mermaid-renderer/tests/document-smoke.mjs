import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const markdown = await readFile(resolve("tests/previous-version.md"), "utf8");
const blocks = [...markdown.matchAll(/```mermaid\s*\r?\n([\s\S]*?)```/g)].map((match) =>
  match[1].trim(),
);

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

const rendered = blocks.map((source, index) => {
  const result = render(source);
  if (!result.ok) {
    throw new Error(`Diagram ${index + 1} failed: ${result.output}`);
  }
  if (!result.output.includes('data-ryen-renderer="0.1.0"')) {
    throw new Error(`Diagram ${index + 1} did not use the Ryen renderer`);
  }
  return { source, svg: result.output };
});

if (process.argv.includes("--write-svg")) {
  const outputDirectory = resolve("tests/rendered");
  await mkdir(outputDirectory, { recursive: true });
  await Promise.all(
    rendered.map(({ svg }, index) =>
      writeFile(resolve(outputDirectory, `diagram-${index + 1}.svg`), svg, "utf8"),
    ),
  );
}

const results = rendered.map(({ source, svg }, index) => ({
  diagram: index + 1,
  direction: source.split(/\r?\n/, 1)[0],
  sourceLength: source.length,
  svgLength: svg.length,
}));

console.log(JSON.stringify({ diagrams: results.length, results }, null, 2));

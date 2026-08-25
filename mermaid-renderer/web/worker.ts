/// <reference lib="webworker" />

import type { WorkerRequest, WorkerResponse } from "./protocol";

type RendererExports = WebAssembly.Exports & {
  memory: WebAssembly.Memory;
  alloc(length: number): number;
  dealloc(pointer: number, capacity: number): void;
  render(pointer: number, length: number): number;
  result_ptr(): number;
  result_len(): number;
  last_ok(): number;
};

let renderer: RendererExports | null = null;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function send(message: WorkerResponse): void {
  self.postMessage(message);
}

async function initialize(wasm: ArrayBuffer): Promise<void> {
  const instance = await WebAssembly.instantiate(wasm, {});
  renderer = instance.instance.exports as RendererExports;
  send({ type: "ready" });
}

function render(source: string): { ok: boolean; text: string } {
  if (!renderer) throw new Error("Renderer is not initialized");
  const input = encoder.encode(source);
  const pointer = renderer.alloc(input.length);
  try {
    new Uint8Array(renderer.memory.buffer, pointer, input.length).set(input);
    renderer.render(pointer, input.length);
    const output = new Uint8Array(
      renderer.memory.buffer,
      renderer.result_ptr(),
      renderer.result_len(),
    );
    return { ok: renderer.last_ok() === 1, text: decoder.decode(output) };
  } finally {
    renderer.dealloc(pointer, input.length);
  }
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const message = event.data;
  if (message.type === "init") {
    try {
      await initialize(message.wasm);
    } catch (error) {
      send({
        type: "result",
        id: -1,
        ok: false,
        error: error instanceof Error ? error.message : String(error),
        elapsedMs: 0,
      });
    }
    return;
  }

  const started = performance.now();
  try {
    const output = render(message.source);
    const elapsedMs = performance.now() - started;
    if (output.ok) {
      send({ type: "result", id: message.id, ok: true, svg: output.text, elapsedMs });
    } else {
      send({ type: "result", id: message.id, ok: false, error: output.text, elapsedMs });
    }
  } catch (error) {
    send({
      type: "result",
      id: message.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
      elapsedMs: performance.now() - started,
    });
  }
};


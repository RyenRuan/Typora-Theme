export type RenderRequest = {
  type: "render";
  id: number;
  source: string;
};

export type InitRequest = {
  type: "init";
  wasm: ArrayBuffer;
};

export type WorkerRequest = InitRequest | RenderRequest;

export type WorkerResponse =
  | { type: "ready" }
  | { type: "result"; id: number; ok: true; svg: string; elapsedMs: number }
  | { type: "result"; id: number; ok: false; error: string; elapsedMs: number };


import type { WorkerResponse } from "./protocol";

declare global {
  interface Window {
    reqnode?: (moduleName: string) => unknown;
    File?: {
      editor?: {
        getNode?: (cid: string) => { get?: (key: string) => unknown } | undefined;
        fences?: { getCm?: (cid: string) => { getValue?: () => string } | undefined };
      };
    };
    __RYEN_MERMAID__?: RyenMermaidController;
  }
}

type PendingRender = {
  resolve: (value: RenderResult) => void;
  source: string;
};

type RenderResult =
  | { ok: true; svg: string; elapsedMs: number }
  | { ok: false; error: string; elapsedMs: number };

const CONFIG = {
  debounceMs: 180,
  fitWidthPx: 1180,
  minReadableScale: 0.62,
  maxNodesHint: 120,
  logPrefix: "[Ryen Mermaid]",
};

const nativeSnapshots = new WeakMap<HTMLElement, string>();

const script = document.currentScript as HTMLScriptElement | null;
const assetBase = new URL(".", script?.src || document.baseURI);

function debug(message: string): void {
  try {
    const fs = window.reqnode as
      | ((moduleName: string) => { appendFileSync?: (path: string, text: string) => void })
      | undefined;
    fs?.("fs")?.appendFileSync?.(
      "C:\\Users\\Ryen\\AppData\\Local\\Temp\\ryen-mermaid-renderer.log",
      `${new Date().toISOString()} ${message}\n`,
    );
  } catch {
    // Diagnostics must never affect rendering.
  }
}

function readBinary(url: URL): Promise<ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open("GET", url.href, true);
    request.responseType = "arraybuffer";
    request.onload = () => {
      if (request.status === 0 || (request.status >= 200 && request.status < 300)) {
        resolve(request.response as ArrayBuffer);
      } else {
        reject(new Error(`Unable to load ${url.href}: HTTP ${request.status}`));
      }
    };
    request.onerror = () => reject(new Error(`Unable to load ${url.href}`));
    request.send();
  });
}

function readText(url: URL): Promise<string> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open("GET", url.href, true);
    request.onload = () => {
      if (request.status === 0 || (request.status >= 200 && request.status < 300)) {
        resolve(request.responseText);
      } else {
        reject(new Error(`Unable to load ${url.href}: HTTP ${request.status}`));
      }
    };
    request.onerror = () => reject(new Error(`Unable to load ${url.href}`));
    request.send();
  });
}

function hashSource(source: string): string {
  let hash = 2166136261;
  for (let index = 0; index < source.length; index += 1) {
    hash ^= source.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(36);
}

function isSupportedSource(source: string): boolean {
  const clean = source
    .replace(/^\s*%%\{[\s\S]*?\}%%\s*/u, "")
    .trimStart();
  return /^(?:flowchart|graph)\s+(?:LR|TD|TB)\b/iu.test(clean);
}

function getFence(preview: HTMLElement): HTMLElement | null {
  return preview.closest<HTMLElement>(".md-fences-advanced, .md-diagram, [cid]");
}

function readSource(preview: HTMLElement): string {
  const fence = getFence(preview);
  const cid = fence?.getAttribute("cid") || "";
  if (cid && window.File?.editor) {
    const nodeText = window.File.editor.getNode?.(cid)?.get?.("text");
    if (typeof nodeText === "string") return nodeText;
    const editorText = window.File.editor.fences?.getCm?.(cid)?.getValue?.();
    if (typeof editorText === "string") return editorText;
  }
  const codeMirror = fence?.querySelector<HTMLElement>(".CodeMirror");
  const editor = (codeMirror as (HTMLElement & { CodeMirror?: { getValue?: () => string } }) | null)
    ?.CodeMirror;
  return editor?.getValue?.() || "";
}

function collapseExpandedMermaidFence(fence: HTMLElement): void {
  const preview = fence.querySelector<HTMLElement>(".md-diagram-panel-preview");
  if (!preview || !isSupportedSource(readSource(preview))) return;

  // Typora marks the newly opened fence as focused. Removing only that
  // transient class returns the document to the rendered-only view while
  // preserving Typora's normal click-to-edit behavior.
  fence.classList.remove("md-focus");
  fence.style.marginBottom = "";
  const active = document.activeElement;
  if (active instanceof HTMLElement && fence.contains(active)) active.blur();
}

function restoreNative(preview: HTMLElement): void {
  const snapshot = nativeSnapshots.get(preview);
  if (snapshot !== undefined && preview.dataset.ryenRendered === "true") {
    preview.innerHTML = snapshot;
  }
  delete preview.dataset.ryenSourceHash;
  delete preview.dataset.ryenRendered;
}

function showCustom(preview: HTMLElement, svg: string, hash: string, elapsedMs: number): void {
  if (!preview.querySelector<HTMLElement>(":scope > .ryen-mermaid-host")) {
    nativeSnapshots.set(preview, preview.innerHTML);
  }
  preview.innerHTML = "";
  const host = document.createElement("div");
  host.className = "ryen-mermaid-host";
  host.contentEditable = "false";
  host.dataset.renderMs = elapsedMs.toFixed(2);
  const root = host.attachShadow({ mode: "open" });
  const style = document.createElement("style");
  style.textContent = `
    :host {
      all: initial;
      display: block;
      width: 100%;
      padding: 8px 0;
      box-sizing: border-box;
      overflow-x: auto;
      overflow-y: hidden;
    }
    .viewport { display: block; margin: 0 auto; overflow: visible; }
    svg { display: block; width: 100%; height: 100%; overflow: visible; }
  `;
  const viewport = document.createElement("div");
  viewport.className = "viewport";
  viewport.innerHTML = svg;
  const renderedSvg = viewport.querySelector<SVGSVGElement>("svg");
  const viewBox = renderedSvg?.viewBox.baseVal;
  if (viewBox && viewBox.width > 0 && viewBox.height > 0) {
    // Keep small diagrams at their natural size. Fit ordinary wide diagrams
    // to the article, but never make truly huge diagrams too small to read;
    // those keep at least 62% scale and scroll inside their own host.
    const displayWidth = Math.min(
      viewBox.width,
      Math.max(CONFIG.fitWidthPx, viewBox.width * CONFIG.minReadableScale),
    );
    viewport.style.width = `${displayWidth}px`;
    viewport.style.aspectRatio = `${viewBox.width} / ${viewBox.height}`;
  }
  root.append(style, viewport);

  preview.appendChild(host);
  preview.dataset.ryenSourceHash = hash;
  preview.dataset.ryenRendered = "true";
}

function pruneNativeAfterCustom(preview: HTMLElement): boolean {
  const host = preview.querySelector<HTMLElement>(":scope > .ryen-mermaid-host");
  if (!host) return false;
  for (const child of Array.from(preview.children)) {
    if (child !== host) child.remove();
  }
  return true;
}

class RyenMermaidController {
  private worker: Worker | null = null;
  private observer: MutationObserver | null = null;
  private ready = false;
  private sequence = 0;
  private pending = new Map<number, PendingRender>();
  private cache = new Map<string, string>();
  private timers = new WeakMap<HTMLElement, number>();
  private startupCollapseCancelled = false;
  private startupInteractionHandler: ((event: Event) => void) | null = null;

async start(): Promise<void> {
    debug(`start assetBase=${assetBase.href}`);
    const [workerSource, wasm] = await Promise.all([
      readText(new URL("worker.js", assetBase)),
      readBinary(new URL("renderer.wasm", assetBase)),
    ]);
    const workerUrl = URL.createObjectURL(new Blob([workerSource], { type: "text/javascript" }));
    this.worker = new Worker(workerUrl);
    URL.revokeObjectURL(workerUrl);
    this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => this.onWorkerMessage(event.data);
    this.worker.onerror = (event) => {
      debug(`worker-error ${event.message}`);
      console.warn(CONFIG.logPrefix, "worker error", event.message);
    };
    this.worker.postMessage({ type: "init", wasm }, [wasm]);

    this.installStartupCollapseGuard();

    this.observer = new MutationObserver((mutations) => {
      const previews = new Set<HTMLElement>();
      for (const mutation of mutations) {
        const target = mutation.target instanceof HTMLElement ? mutation.target : mutation.target.parentElement;
        const preview = target?.closest<HTMLElement>(".md-diagram-panel-preview");
        if (preview) previews.add(preview);
        for (const node of Array.from(mutation.addedNodes)) {
          if (!(node instanceof HTMLElement)) continue;
          if (node.matches(".md-diagram-panel-preview")) previews.add(node);
          node.querySelectorAll<HTMLElement>(".md-diagram-panel-preview").forEach((item) => previews.add(item));
        }
      }
      previews.forEach((preview) => this.schedule(preview));
    });
    this.observer.observe(document.getElementById("write") || document.body, {
      childList: true,
      subtree: true,
      characterData: true,
    });
    document
      .querySelectorAll<HTMLElement>(".md-diagram-panel-preview")
      .forEach((preview) => this.schedule(preview, 0));
    debug(`observer-start previews=${document.querySelectorAll(".md-diagram-panel-preview").length}`);
  }

  stop(): void {
    this.observer?.disconnect();
    this.worker?.terminate();
    this.pending.clear();
    if (this.startupInteractionHandler) {
      document.removeEventListener("mousedown", this.startupInteractionHandler, true);
      document.removeEventListener("keydown", this.startupInteractionHandler, true);
      this.startupInteractionHandler = null;
    }
    document
      .querySelectorAll<HTMLElement>(".md-diagram-panel-preview[data-ryen-rendered]")
      .forEach(restoreNative);
  }

  private installStartupCollapseGuard(): void {
    const handler = (event: Event) => {
      const target = event.target;
      if (target instanceof HTMLElement && target.closest(".md-fences-advanced, .md-diagram")) {
        this.startupCollapseCancelled = true;
      }
    };
    this.startupInteractionHandler = handler;
    document.addEventListener("mousedown", handler, true);
    document.addEventListener("keydown", handler, true);
  }

  private collapseIfNeeded(preview: HTMLElement): void {
    if (this.startupCollapseCancelled) return;
    const fence = getFence(preview);
    if (fence) collapseExpandedMermaidFence(fence);
  }

  private schedule(preview: HTMLElement, delay = CONFIG.debounceMs): void {
    const existing = this.timers.get(preview);
    if (existing) window.clearTimeout(existing);
    const timer = window.setTimeout(() => {
      this.timers.delete(preview);
      void this.update(preview);
    }, delay);
    this.timers.set(preview, timer);
  }

  private async update(preview: HTMLElement): Promise<void> {
    if (!preview.isConnected || preview.closest("#componenet")) {
      debug("update-skip disconnected-or-template");
      return;
    }
    const fence = getFence(preview);
    const language = fence?.getAttribute("lang")?.toLowerCase();
    if (language && language !== "mermaid") return;
    const source = readSource(preview);
    debug(`update sourceLength=${source.length}`);
    if (!isSupportedSource(source)) {
      debug("update-native-unsupported");
      if (preview.dataset.ryenRendered) restoreNative(preview);
      return;
    }
    const hash = hashSource(source);
    if (preview.dataset.ryenSourceHash === hash && preview.dataset.ryenRendered === "true") {
      if (pruneNativeAfterCustom(preview)) {
        this.collapseIfNeeded(preview);
        return;
      }
      debug("custom-host-replaced-replay");
    }
    const cached = this.cache.get(hash);
    if (cached) {
      showCustom(preview, cached, hash, 0);
      this.collapseIfNeeded(preview);
      return;
    }
    const result = await this.render(source);
    if (!preview.isConnected || hashSource(readSource(preview)) !== hash) return;
    if (result.ok) {
      debug(`update-custom-ok elapsedMs=${result.elapsedMs.toFixed(2)}`);
      this.cache.set(hash, result.svg);
      if (this.cache.size > 80) this.cache.delete(this.cache.keys().next().value as string);
      showCustom(preview, result.svg, hash, result.elapsedMs);
      this.collapseIfNeeded(preview);
    } else {
      debug(`update-native-fallback error=${result.error}`);
      restoreNative(preview);
      console.info(CONFIG.logPrefix, "native fallback:", result.error);
    }
  }

  private render(source: string): Promise<RenderResult> {
    if (!this.worker || !this.ready) {
      return new Promise((resolve) => window.setTimeout(() => this.render(source).then(resolve), 30));
    }
    const id = ++this.sequence;
    return new Promise((resolve) => {
      this.pending.set(id, { resolve, source });
      this.worker?.postMessage({ type: "render", id, source });
    });
  }

  private onWorkerMessage(message: WorkerResponse): void {
    if (message.type === "ready") {
      this.ready = true;
      return;
    }
    if (message.id === -1) {
      console.warn(CONFIG.logPrefix, message.ok ? "unexpected response" : message.error);
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) return;
    this.pending.delete(message.id);
    pending.resolve(
      message.ok
        ? { ok: true, svg: message.svg, elapsedMs: message.elapsedMs }
        : { ok: false, error: message.error, elapsedMs: message.elapsedMs },
    );
  }
}

if (!window.__RYEN_MERMAID__) {
  const controller = new RyenMermaidController();
  window.__RYEN_MERMAID__ = controller;
  const launch = () =>
    controller.start().catch((error) => {
      debug(`start-error ${error instanceof Error ? error.message : String(error)}`);
      console.warn(CONFIG.logPrefix, error);
    });
  debug("script-loaded");
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", launch, { once: true });
  else launch();
}

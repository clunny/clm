// A small DOM builder. Repo text only ever goes in as text nodes, never as HTML.

export type Child = Node | string | number | null | undefined | false | Child[];
type Attrs = Record<string, string | number | boolean | null | undefined | EventListener>;

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  setAttrs(e, attrs);
  append(e, children);
  return e;
}

const SVG = 'http://www.w3.org/2000/svg';

export function svg(tag: string, attrs: Attrs = {}, ...children: Child[]): SVGElement {
  const e = document.createElementNS(SVG, tag);
  setAttrs(e, attrs);
  append(e, children);
  return e;
}

function setAttrs(e: Element, attrs: Attrs) {
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (typeof v === 'function') e.addEventListener(k.replace(/^on/, ''), v);
    else e.setAttribute(k, v === true ? '' : String(v));
  }
}

export function append(e: Node, children: Child[]) {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(e, c);
    else e.appendChild(c instanceof Node ? c : document.createTextNode(String(c)));
  }
}

/** An element with a class, the most common shape. */
export function div(cls: string, ...children: Child[]): HTMLDivElement {
  return el('div', { class: cls }, ...children);
}

export function chip(cls: string, text: string): HTMLSpanElement {
  return el('span', { class: `chip ${cls}` }, text);
}

export function code(text: string): HTMLElement {
  return el('code', {}, text);
}

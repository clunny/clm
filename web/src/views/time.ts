// Relative times, from Unix seconds, and a record's time as people read it.

/** A ledger record's RFC 3339 time as `YYYY-MM-DD HH:MM UTC`; the text itself when it
 * isn't a time. */
export function when(at: string): string {
  const t = new Date(at);
  return Number.isNaN(t.getTime()) ? at : `${t.toISOString().slice(0, 16).replace('T', ' ')} UTC`;
}

function span(seconds: number): string {
  const min = Math.floor(seconds / 60);
  if (min < 60) return `${min} min`;
  const h = Math.floor(min / 60);
  if (h < 48) return `${h} h`;
  return `${Math.floor(h / 24)} d`;
}

export function ago(at: number, now: number): string {
  const s = now - at;
  return s < 60 ? 'just now' : `${span(s)} ago`;
}

/** How long until a claim lapses: "3 h", "3 h 22 m" or "12 min". */
export function lapsesIn(lapsesAt: number, now: number): string {
  const s = lapsesAt - now;
  if (s <= 0) return 'now';
  const min = Math.floor(s / 60);
  if (min < 60) return `${min} min`;
  const h = Math.floor(min / 60);
  const m = min % 60;
  return m ? `${h} h ${m} m` : `${h} h`;
}

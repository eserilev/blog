/* Stand-ins for the editor and the Now box, until steps 5 and 6.
   md() escapes all input text and emits a fixed set of tags only.
   Step 5 replaces it with the WASM render(). Then this file goes away. */

const DRAFT_MD = `# ePBS from a client's perspective

Glamsterdam splits the block in two. The proposer commits to a builder bid. The builder reveals the payload later.

## What changes for fork choice

- Fork choice now tracks **payload presence**, not only the block.
- The payload timeliness committee (PTC) votes on whether the payload arrived on time.
- An *empty* slot is now different from a *missed* slot.

## First pass at the type

\`\`\`rust
pub enum PayloadStatus {
    Pending,
    Revealed { block_hash: Hash256 },
    Withheld,
}
\`\`\`

> TODO: add the fork choice diagram before publishing.`;

/* Tiny markdown renderer. It covers headings, paragraphs, lists, quotes, fences, rules,
   bold, italic, inline code, and links. A real build swaps in a full parser. */
function md(src) {
  const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  const inline = s => esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+|#[^)\s]*)\)/g, '<a href="$2">$1</a>');
  const block = /^(#{1,4}\s|```|>|[-*]\s|\d+\.\s|---+\s*$)/;
  const lines = src.replace(/\r/g, '').split('\n');
  const out = [];
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (l.startsWith('```')) {
      const lang = l.slice(3).trim(), buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) buf.push(lines[i++]);
      i++;
      out.push(`<pre data-lang="${esc(lang)}"><code>${esc(buf.join('\n'))}</code></pre>`);
      continue;
    }
    const h = l.match(/^(#{1,4})\s+(.*)/);
    if (h) { out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`); i++; continue; }
    if (/^>/.test(l)) {
      const buf = [];
      while (i < lines.length && /^>/.test(lines[i])) buf.push(lines[i++].replace(/^>\s?/, ''));
      out.push(`<blockquote><p>${inline(buf.join(' '))}</p></blockquote>`);
      continue;
    }
    const list = /^[-*]\s+/.test(l) ? ['ul', /^[-*]\s+/] : /^\d+\.\s+/.test(l) ? ['ol', /^\d+\.\s+/] : null;
    if (list) {
      const buf = [];
      while (i < lines.length && list[1].test(lines[i])) buf.push(`<li>${inline(lines[i++].replace(list[1], ''))}</li>`);
      out.push(`<${list[0]}>${buf.join('')}</${list[0]}>`);
      continue;
    }
    if (/^---+\s*$/.test(l)) { out.push('<hr>'); i++; continue; }
    if (!l.trim()) { i++; continue; }
    const buf = [];
    while (i < lines.length && lines[i].trim() && !block.test(lines[i])) buf.push(lines[i++]);
    out.push(`<p>${inline(buf.join(' '))}</p>`);
  }
  return out.join('\n');
}

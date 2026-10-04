/* Sample data and a stand-in markdown renderer.
   Temporary. Step 2 replaces the data with the API and shows server-rendered body_html.
   Step 5 replaces md() with the WASM render(). Then this file goes away.
   md() escapes all input text and emits a fixed set of tags only. */

const POSTS = [
  { id: 'bal', title: 'Block-level access lists and parallel execution', slug: 'block-level-access-lists.md',
    date: '2026-09-28', kind: 'eth', topic: 'Ethereum', tags: ['glamsterdam', 'eip-7928', 'clients'],
    mins: 9, words: 1980, vis: 'public', isNew: true,
    excerpt: 'EIP-7928 makes each block declare the accounts and storage slots it touches. What this gives clients, and what it costs builders.' },
  { id: 'ssz', title: 'Zero-copy SSZ decoding in Rust', slug: 'zero-copy-ssz.md',
    date: '2026-09-14', kind: 'rust', topic: 'Rust', tags: ['rust', 'ssz', 'performance'],
    mins: 12, words: 2640, vis: 'public', isNew: true,
    excerpt: 'Decoding beacon state without allocations: borrowing from the input buffer, the lifetime problems that follow, and where the change paid off.' },
  { id: 'ui', title: 'What classic WoW addons got right about interfaces', slug: 'classic-wow-interfaces.md',
    date: '2026-09-02', kind: 'wow', topic: 'Classic WoW', tags: ['design', 'wow'],
    mins: 7, words: 1530, vis: 'public',
    excerpt: 'Raid frames and threat meters solved information density problems that many dashboards still get wrong.' },
  { id: 'bjj', title: 'Four years of jiu jitsu', slug: 'four-years-of-jiu-jitsu.md',
    date: '2026-08-19', kind: 'bjj', topic: 'Jiu jitsu', tags: ['bjj'],
    mins: 5, words: 1120, vis: 'public',
    excerpt: 'Notes on training consistently while working on a protocol, and on what the mat teaches about hard problems.' },
  { id: 'surf', title: 'Surf log, summer 2026', slug: 'surf-log-summer-2026.md',
    date: '2026-07-30', kind: 'surf', topic: 'Surf', tags: ['surf'],
    mins: 3, words: 610, vis: 'public',
    excerpt: 'Conditions, a new board, and why I keep my mornings free of calls.' },
  { id: 'split', title: 'Splitboarding: end of season notes', slug: 'end-of-season.md',
    date: '2026-04-11', kind: 'snow', topic: 'Snowboarding', tags: ['snow', 'backcountry'],
    mins: 6, words: 1290, vis: 'public',
    excerpt: 'Route, gear, and conditions from the last backcountry tour of the season.' },
  { id: 'epbs', title: "ePBS from a client's perspective", slug: 'epbs-client-view.md',
    date: '2026-10-01', kind: 'eth', topic: 'Ethereum', tags: ['glamsterdam', 'epbs'],
    mins: 4, words: 412, vis: 'private', draft: true,
    excerpt: 'Draft. Fork choice now has to track the payload, not only the block.' },
];

const POST_MD = `An execution client receives a block today with no information about the state it touches. It must execute the transactions in order, because any transaction can depend on the result of the one before it.

EIP-7928 changes this. Each block carries a **block-level access list** (BAL): every account and storage slot that the block reads or writes, with the values after each transaction.

## What clients gain

- Clients can load state from disk in parallel, before execution starts.
- Transactions with separate access sets can execute in parallel.
- Some sync paths can apply state changes without full re-execution.

## What it costs

The list adds data to every block. Builders must produce it, and clients must make sure that it matches the result of execution. A block with a wrong list is invalid.

## A rough shape

This is not the spec type. It is a simplified version for discussion:

\`\`\`rust
pub struct AccountChanges {
    pub address: Address,
    pub storage_writes: Vec<SlotWrites>,
    pub storage_reads: Vec<B256>,
    pub balance_changes: Vec<(u16, U256)>,
}
\`\`\`

## Why it matters

Block validation time is one of the main limits on the gas limit. If clients validate blocks faster, the network can raise the limit safely.`;

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

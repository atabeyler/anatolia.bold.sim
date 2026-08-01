// Regression tests for downloadPDF()'s DOM chunking helper. A user reported
// the Report panel's PDF export freezing/hanging the tab on a long-running
// simulation -- buildReportHtml() built one giant HTML string (Individuals/
// Notable Events/Migration History tables scale with simulation length, the
// Individuals one unboundedly since dead individuals never leave the array)
// and html2canvas rasterized the whole thing as one canvas at scale:2. This
// splits that HTML into small per-section chunks (further batched for the
// Individuals table) so no single html2canvas call ever rasterizes an
// unbounded amount of content, without truncating any data out of the PDF.
import { describe, it, expect } from 'vitest';
import { splitIntoSectionChunks } from './ReportPanel';

function parse(html: string): HTMLElement {
  const div = document.createElement('div');
  div.innerHTML = html;
  return div;
}

describe('splitIntoSectionChunks', () => {
  it('starts a new chunk at each HTML comment marker', () => {
    const root = parse(`
      <!-- SECTION A -->
      <div>a1</div>
      <table><tr><td>a2</td></tr></table>
      <!-- SECTION B -->
      <p>b1</p>
    `);
    const chunks = splitIntoSectionChunks(root);
    expect(chunks.length).toBe(2);
    expect(chunks[0].map(n => (n as HTMLElement).tagName)).toEqual(['DIV', 'TABLE']);
    expect(chunks[1].map(n => (n as HTMLElement).tagName)).toEqual(['P']);
  });

  it('drops whitespace-only text nodes so they never form empty chunks', () => {
    const root = parse(`
      <!-- SECTION A -->


      <div>content</div>


      <!-- SECTION B -->


    `);
    const chunks = splitIntoSectionChunks(root);
    // The trailing comment has only whitespace after it -- must not produce
    // a second, empty chunk.
    expect(chunks.length).toBe(1);
    expect(chunks[0].length).toBe(1);
  });

  it('treats content before the first comment as its own leading chunk', () => {
    const root = parse(`<div>no marker yet</div><!-- SECTION A --><p>marked</p>`);
    const chunks = splitIntoSectionChunks(root);
    expect(chunks.length).toBe(2);
    expect((chunks[0][0] as HTMLElement).textContent).toBe('no marker yet');
    expect((chunks[1][0] as HTMLElement).textContent).toBe('marked');
  });

  it('keeps a section spanning several sibling elements together in one chunk', () => {
    // Mirrors a real section shape: a title bar, a truncation-note
    // paragraph (when present), and the actual table, all as direct
    // siblings between one pair of comment markers.
    const root = parse(`
      <!-- BİREYLER -->
      <div>Bireyler</div>
      <table id="rpt-individuals-table"><tr><th>Name</th></tr><tr><td>Alice</td></tr></table>
      <!-- SONRAKI -->
      <div>next</div>
    `);
    const chunks = splitIntoSectionChunks(root);
    expect(chunks.length).toBe(2);
    expect(chunks[0].length).toBe(2);
    const table = chunks[0][1] as HTMLTableElement;
    expect(table.id).toBe('rpt-individuals-table');
  });
});

/**
 * The agent owns the prose, but it is still untrusted input at this boundary.
 * This deliberately small renderer emits only escaped text and a short allow-list
 * of Markdown constructs; it never forwards agent HTML into the document.
 */
function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function safeUrl(value: string): string | null {
  try {
    const url = new URL(value, 'https://msc.invalid');
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null;
  } catch {
    return null;
  }
}

function inline(value: string): string {
  const escaped = escapeHtml(value);
  return escaped.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label, rawUrl) => {
    const url = safeUrl(rawUrl);
    return url ? `<a href="${escapeHtml(url)}" rel="noopener noreferrer">${label}</a>` : label;
  });
}

/** Render Markdown as a safe, intentionally limited HTML fragment. */
export function renderMarkdown(markdown: string): string {
  const lines = markdown.replace(/\r\n/g, '\n').split('\n');
  const output: string[] = [];
  let inList = false;
  let inCode = false;

  const closeList = () => {
    if (inList) output.push('</ul>');
    inList = false;
  };

  for (const line of lines) {
    if (line.startsWith('```')) {
      closeList();
      output.push(inCode ? '</code></pre>' : '<pre><code>');
      inCode = !inCode;
      continue;
    }
    if (inCode) {
      output.push(`${escapeHtml(line)}\n`);
      continue;
    }
    const heading = /^(#{1,3})\s+(.+)$/.exec(line);
    if (heading) {
      closeList();
      output.push(`<h${heading[1].length}>${inline(heading[2])}</h${heading[1].length}>`);
      continue;
    }
    const item = /^[-*]\s+(.+)$/.exec(line);
    if (item) {
      if (!inList) output.push('<ul>');
      inList = true;
      output.push(`<li>${inline(item[1])}</li>`);
      continue;
    }
    closeList();
    if (line.trim()) output.push(`<p>${inline(line)}</p>`);
  }
  closeList();
  if (inCode) output.push('</code></pre>');
  return output.join('');
}

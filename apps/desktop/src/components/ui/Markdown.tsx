import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * A compact, hand-styled Markdown renderer for issue / PR / comment / review
 * bodies. We deliberately avoid a heavyweight typography plugin and instead map
 * each element onto the app's design tokens via `components`:
 *   - readable line-height, modest heading scale
 *   - code / pre on surface-2 with a hairline border
 *   - links in the accent color (opened in the system browser, since these are
 *     GitHub-authored bodies that may point anywhere)
 *   - lists / blockquotes / tables tuned to sit inside hairline cards
 *
 * react-markdown does not render raw HTML by default, so bodies are safe.
 */

function openExternal(url: string) {
  void openUrl(url).catch(() => {});
}

export function Markdown({ children }: { children: string }) {
  return (
    <div className="text-[13px] leading-relaxed text-text [&>*:first-child]:mt-0 [&>*:last-child]:mb-0">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => (
            <h1 className="mb-2 mt-4 text-[18px] font-semibold tracking-tight text-text">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="mb-2 mt-4 text-[16px] font-semibold tracking-tight text-text">
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="mb-1.5 mt-3 text-[14px] font-semibold text-text">
              {children}
            </h3>
          ),
          h4: ({ children }) => (
            <h4 className="mb-1.5 mt-3 text-[13px] font-semibold text-text">
              {children}
            </h4>
          ),
          p: ({ children }) => <p className="my-2">{children}</p>,
          a: ({ href, children }) => (
            <a
              href={href}
              onClick={(e) => {
                e.preventDefault();
                if (href) openExternal(href);
              }}
              className="text-accent underline-offset-2 hover:underline outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              {children}
            </a>
          ),
          ul: ({ children }) => (
            <ul className="my-2 list-disc space-y-1 pl-5 marker:text-text-faint">
              {children}
            </ul>
          ),
          ol: ({ children }) => (
            <ol className="my-2 list-decimal space-y-1 pl-5 marker:text-text-faint">
              {children}
            </ol>
          ),
          li: ({ children }) => <li className="pl-0.5">{children}</li>,
          blockquote: ({ children }) => (
            <blockquote className="my-2 border-l-2 border-border pl-3 text-text-muted">
              {children}
            </blockquote>
          ),
          code: ({ className, children }) => {
            const isBlock = /language-/.test(className ?? "");
            if (isBlock) {
              return (
                <code className="font-mono text-[12px]">{children}</code>
              );
            }
            return (
              <code className="rounded-[--radius-sm] border border-border bg-surface-2 px-1 py-0.5 font-mono text-[12px] text-text">
                {children}
              </code>
            );
          },
          pre: ({ children }) => (
            <pre className="my-2 overflow-x-auto rounded-[--radius] border border-border bg-surface-2 p-3 text-[12px] leading-relaxed text-text">
              {children}
            </pre>
          ),
          hr: () => <hr className="my-4 border-border" />,
          img: ({ src, alt }) => (
            <img
              src={typeof src === "string" ? src : undefined}
              alt={alt ?? ""}
              loading="lazy"
              className="my-2 max-w-full rounded-[--radius] border border-border"
            />
          ),
          table: ({ children }) => (
            <div className="my-2 overflow-x-auto">
              <table className="w-full border-collapse text-[12px]">
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border border-border bg-surface-2 px-2 py-1 text-left font-medium text-text">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="border border-border px-2 py-1 text-text-muted">
              {children}
            </td>
          ),
          strong: ({ children }) => (
            <strong className="font-semibold text-text">{children}</strong>
          ),
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}

export default Markdown;

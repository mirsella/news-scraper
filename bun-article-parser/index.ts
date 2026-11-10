import { Hono } from "hono";
import { capture, exception, log } from "./telemetry";
import { extract, extractFromHtml } from "@extractus/article-extractor";
import { Readability } from "@mozilla/readability";
import { DOMParser } from "linkedom";

const app = new Hono();
app.use("*", async (c, next) => {
  if (!["/fetch", "/parse"].includes(c.req.path)) return next();
  const started = performance.now();
  await next();
  const properties = { operation: c.req.path, method: c.req.method, status: c.res.status,
    success: c.res.status < 400, duration_ms: Math.round(performance.now() - started) };
  capture("parser_request_completed", properties);
  log("Parser request completed", "INFO", { operation: c.req.path, status: String(c.res.status) });
});

type ArticleData = {
  url: string;
  title: string;
  description: string;
  image: string;
  author: string;
  favicon: string;
  content: string;
  published: string;
  source: string;
  links: string[];
  ttr: number;
  type: string;
};

function metaContent(doc: any, selector: string): string {
  return doc.querySelector(selector)?.getAttribute("content")?.trim() ?? "";
}

function resolveUrl(value: string, base: string): string {
  if (!value) return "";
  try {
    return new URL(value, base || undefined).toString();
  } catch {
    return value;
  }
}

function textLength(html: string): number {
  const doc = new DOMParser().parseFromString(html, "text/html");
  return doc.documentElement.textContent?.trim().length ?? 0;
}

// extractus removes unknown elements together with their children. Some sites
// (notably La Voix du Nord and RTL) put article text in custom elements, which
// makes the library return null even though Readability found the article.
function fallbackExtract(html: string, inputUrl = ""): ArticleData | null {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const canonical = doc.querySelector('link[rel="canonical"]')?.getAttribute("href") ?? "";
  const ogUrl = metaContent(doc, 'meta[property="og:url"]');
  const url = inputUrl || canonical || ogUrl;
  if (url) {
    const base = doc.createElement("base");
    base.setAttribute("href", url);
    doc.head?.appendChild(base);
  }

  for (const element of doc.querySelectorAll("script,style,noscript,iframe,form,button,input,svg,canvas")) {
    element.remove();
  }

  let article: ReturnType<Readability["parse"]>;
  try {
    article = new Readability(doc, { charThreshold: 200, keepClasses: true }).parse();
  } catch (err) {
    exception(err, "readability");
    return null;
  }
  if (!article?.content || !article.textContent || article.textContent.trim().length < 200) {
    return null;
  }

  const contentDoc = new DOMParser().parseFromString(article.content, "text/html");
  for (const element of contentDoc.querySelectorAll("script,style,noscript,iframe,form,button,input,svg,canvas")) {
    element.remove();
  }
  for (const element of contentDoc.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      if (attribute.name.toLowerCase().startsWith("on")) element.removeAttribute(attribute.name);
    }
  }

  const title = metaContent(doc, 'meta[property="og:title"]') || article.title || doc.title?.trim() || "";
  const description =
    metaContent(doc, 'meta[name="description"]') ||
    metaContent(doc, 'meta[property="og:description"]') ||
    article.excerpt ||
    "";
  const image = resolveUrl(metaContent(doc, 'meta[property="og:image"]'), url);
  const author = metaContent(doc, 'meta[name="author"]') || article.byline || "";
  const published =
    metaContent(doc, 'meta[property="article:published_time"]') ||
    metaContent(doc, 'meta[name="date"]') ||
    article.publishedTime ||
    "";
  const favicon = resolveUrl(doc.querySelector('link[rel~="icon"]')?.getAttribute("href") ?? "", url);
  let source = article.siteName || "";
  if (!source && url) {
    try {
      source = new URL(url).hostname.replace(/^www\./, "");
    } catch {
      source = "";
    }
  }
  const words = article.textContent.trim().split(/\s+/).length;

  return {
    url,
    title,
    description,
    image,
    author,
    favicon,
    content: contentDoc.documentElement.innerHTML,
    published,
    source,
    links: [],
    ttr: Math.max(1, Math.round(words / 300)),
    type: "",
  };
}

async function parseArticle(html: string, url = ""): Promise<ArticleData | null> {
  try {
    const extracted = await extractFromHtml(html, url);
    if (extracted?.content && textLength(extracted.content) >= 200) {
      return extracted as ArticleData;
    }
  } catch (err) {
    exception(err, "extractFromHtml");
  }
  return fallbackExtract(html, url);
}

app.get("/fetch", async (c) => {
  try {
    const url = c.req.query("url");
    if (!url) {
      c.status(400);
      return c.json({ message: "url is required" });
    }
    const data = await extract(url, undefined, (target) => fetch(target, {
      signal: AbortSignal.timeout(5000),
      headers: {
        "user-agent":
          "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.3",
      },
    }));
    return c.json(data);
  } catch (err) {
    c.status(500);
    exception(err, "fetch");
    console.error("Article fetch failed");
    return c.json({ message: err.toString() });
  }
});

app.post("/parse", async (c) => {
  try {
    const body = await c.req.text();
    if (!body) {
      console.log("no body provided");
      c.status(400);
      return c.json({ message: "no body provided" });
    }

    const data = await parseArticle(body, c.req.query("url") ?? "");
    if (data == null) {
      c.status(500);
    }
    return c.json(data);
  } catch (err) {
    c.status(500);
    exception(err, "parse");
    console.error("Article parse failed");
    return c.json({ message: err.toString() });
  }
});

const port = process.env.PORT || 8081;
export default {
  port: port,
  fetch: app.fetch,
};
console.log("running on " + port);

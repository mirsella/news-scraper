import { describe, expect, test } from "bun:test";
import app from "./index";

const paragraph = "A community garden opened this week. Volunteers planted trees and vegetables, and neighbors gathered to celebrate the new green space. ";
const html = `<!doctype html><html lang="en"><head>
  <title>Neighbors open a new community garden</title>
  <meta property="og:type" content="article">
  <meta property="og:title" content="Neighbors open a new community garden">
  <meta property="og:url" content="https://example.com/community-garden">
  <meta property="article:published_time" content="2026-11-09T10:00:00Z">
  </head><body><article><h1>Neighbors open a new community garden</h1>
  <p>${paragraph.repeat(10)}</p></article></body></html>`;

describe("article parser", () => {
  test("requires a URL for fetch", async () => {
    const response = await app.fetch(new Request("http://localhost/fetch"));
    expect(response.status).toBe(400);
    expect(await response.json()).toEqual({ message: "url is required" });
  });

  test("requires HTML for parse", async () => {
    const response = await app.fetch(new Request("http://localhost/parse", { method: "POST" }));
    expect(response.status).toBe(400);
  });

  test("extracts a complete article from HTML", async () => {
    const response = await app.fetch(new Request("http://localhost/parse", {
      method: "POST", body: html, headers: { "Content-Type": "text/html" },
    }));
    expect(response.status).toBe(200);
    const article = await response.json();
    expect(article.title).toBe("Neighbors open a new community garden");
    expect(article.content).toContain("Volunteers planted trees");
    expect(article.published).toBe("2026-11-09T10:00:00.000Z");
  });

  test("falls back to Readability when extractus drops custom elements", async () => {
    const customHtml = `<!doctype html><html lang="fr"><head>
      <title>Une bonne nouvelle</title>
      <link rel="canonical" href="https://example.fr/bonne-nouvelle">
      <meta property="og:title" content="Une bonne nouvelle">
      </head><body><main><article>
      <r-article--header><h1>Une bonne nouvelle</h1></r-article--header>
      <r-article--chapo><p>${paragraph.repeat(3)}</p></r-article--chapo>
      <r-article--body><p>${paragraph.repeat(8)}</p></r-article--body>
      </article></main></body></html>`;
    const response = await app.fetch(new Request("http://localhost/parse", {
      method: "POST", body: customHtml, headers: { "Content-Type": "text/html" },
    }));
    expect(response.status).toBe(200);
    const article = await response.json();
    expect(article.url).toBe("https://example.fr/bonne-nouvelle");
    expect(article.content).toContain("Volunteers planted trees");
  });

  test("fetches and extracts an article with the v9 fetch callback", async () => {
    let userAgent: string | null = null;
    const source = Bun.serve({
      hostname: "127.0.0.1", port: 0,
      fetch(request) {
        userAgent = request.headers.get("user-agent");
        return new Response(html, { headers: { "Content-Type": "text/html" } });
      },
    });
    try {
      const response = await app.fetch(new Request(`http://localhost/fetch?url=${encodeURIComponent(source.url.href)}`));
      expect(response.status).toBe(200);
      const article = await response.json();
      expect(article.title).toBe("Neighbors open a new community garden");
      expect(article.content).toContain("Volunteers planted trees");
      expect(userAgent).toContain("Mozilla/5.0");
    } finally {
      await source.stop(true);
    }
  });
});

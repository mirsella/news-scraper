import { Hono, Context } from "https://deno.land/x/hono@v3.5.4/mod.ts";
import { extract, extractFromHtml } from "npm:@extractus/article-extractor";

const app = new Hono();

app.get("/fetch", async (c: Context) => {
  const url = c.req.query("url");
  if (!url) {
    c.status(400);
    return c.json({ message: "url is required" });
  }
  try {
    const data = await extract(url);
    return c.json(data);
  } catch (err) {
    c.status(500);
    return c.json({ message: err.toString() });
  }
});

app.get("/parse", async (c: Context) => {
  const body = await c.req.text();
  if (!body) {
    c.status(400);
    return c.json({ message: "html body is required" });
  }

  try {
    const data = await extractFromHtml(body);
    return c.json(data);
  } catch (err) {
    c.status(500);
    return c.json({ message: err.toString() });
  }
});

Deno.serve({ port: 8080 }, app.fetch);

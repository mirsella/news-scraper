import { Hono } from "https://deno.land/x/hono/mod.ts";

import { extract, extractFromHtml } from "npm:@extractus/article-extractor";

const app = new Hono();

const meta = {
  service: "article-parser",
  lang: "typescript",
  server: "hono",
  platform: "deno",
};

app.get("/fetch", async (c) => {
  const url = c.req.query("url");
  if (!url) {
    return c.json(meta);
  }
  try {
    const data = await extract(url);
    return c.json({
      error: 0,
      message: "article has been extracted successfully",
      data,
      meta,
    });
  } catch (err) {
    return c.json({
      error: 1,
      message: err.message,
      data: null,
      meta,
    });
  }
});

app.post("/parse", async (c) => {
  const body = await c.req.text();

  try {
    const data = await extractFromHtml(body);
    return c.json({
      error: 0,
      message: "article has been extracted successfully",
      data,
      meta,
    });
  } catch (err) {
    return c.json({
      error: 1,
      message: err.message,
      data: null,
      meta,
    });
  }
});

// app.listen(3100).then(() => {
//   console.log("Server is running at http://localhost:3100");
// });
Deno.serve({ port: 3100 }, app.fetch);

import { Context, Hono } from "hono";
import { extract, extractFromHtml } from "@extractus/article-extractor";

const app = new Hono();

app.get("/fetch", async (c: Context) => {
  try {
    const url = c.req.query("url");
    if (!url) {
      c.status(400);
      return c.json({ message: "url is required" });
    }
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 5000);
    const data = await extract(url, undefined, {
      signal: controller.signal,
      headers: {
        "user-agent":
          "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.3",
      },
    });
    return c.json(data);
  } catch (err) {
    c.status(500);
    console.error(err);
    return c.json({ message: err.toString() });
  }
});

app.post("/parse", async (c: Context) => {
  try {
    const body = await c.req.text();
    if (!body) {
      console.log("no body provided" + body);
      c.status(400);
      return c.json({ message: "no body provided" });
    }

    const data = await extractFromHtml(body);
    if (data == null) {
      c.status(500);
    }
    return c.json(data);
  } catch (err) {
    c.status(500);
    console.error(err);
    return c.json({ message: err.toString() });
  }
});

const port = process.env.PORT || 8081;
export default {
  port: port,
  fetch: app.fetch,
};
console.log("running on " + port);

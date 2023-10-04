import express from "express";
import { extract } from "@extractus/article-extractor";

const app = express();

app.get("/fetch", async (req, res) => {
  const url = req.query.url;
  if (!url) {
    res.status(400);
    return res.json("no url provided");
  }
  try {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 5000);
    const data = await extract(url, undefined, { signal: controller.signal });
    return res.json(data);
  } catch (err) {
    res.status(500);
    console.error(err);
    return res.json(err);
  }
});

app.post("/parse", async (req, res) => {
  const body = req.body;
  if (!body) {
    res.status(400);
    return res.json("no body provided");
  }
  try {
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

app.listen(8080, () => {
  console.log("Server is running at http://localhost:8080");
});

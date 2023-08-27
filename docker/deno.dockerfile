FROM denoland/deno:alpine-1.36.3
WORKDIR /app
COPY ../deno-article-parser/ /app
EXPOSE 8080
CMD ["deno", "task", "start"]

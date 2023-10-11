FROM denoland/deno:alpine-1.36.3
WORKDIR /app
COPY ../deno-article-parser/ /app
RUN apk add --no-cache curl
EXPOSE 8080
CMD ["deno", "task", "run"]


# FROM node:20.8.0-alpine3.18
# WORKDIR /app
# COPY ../node-article-parser/ /app
# RUN npm i
# EXPOSE 8080
# CMD ["npm", "run", "start"]

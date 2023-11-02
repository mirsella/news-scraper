FROM node:21.0-alpine3.18

WORKDIR /app


COPY ../nuxt .

RUN apk add --no-cache curl
# pnpm not in alpine repo yet...
RUN npm i -g pnpm
RUN pnpm i && pnpm build

CMD ["pnpm", "nuxt", "start"]

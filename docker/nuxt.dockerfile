FROM node:21.0-alpine3.17

ARG NUXT_BASE_URL
ENV NUXT_BASE_URL=$NUXT_BASE_URL

WORKDIR /app

COPY ../nuxt .

RUN apk add --no-cache curl
# pnpm not in alpine repo yet...
RUN npm i -g pnpm && pnpm i && pnpm build

CMD ["node", ".output/server/index.mjs"]

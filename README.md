ngrok config:

```yml
tunnels:
  surrealdb:
    addr: 8000
    labels:
      - edge=lmp
  nuxt:
    addr: 3000
    labels:
      - edge=lmp
```

localtunnel can also be used, but visiting the site requires to enter the client ip the first time
lmp ip 141.255.129.243

need to look into cloudflared

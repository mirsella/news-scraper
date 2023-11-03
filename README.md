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

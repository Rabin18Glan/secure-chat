FROM caddy:2.10-alpine
COPY deploy/Caddyfile /etc/caddy/Caddyfile
COPY client/apps/website/dist /srv
EXPOSE 80 443

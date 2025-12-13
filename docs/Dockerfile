FROM caddy:2-alpine

COPY Caddyfile /etc/caddy/Caddyfile
COPY index.html /srv/index.html
COPY app.js /srv/app.js
COPY styles.css /srv/styles.css
COPY favicon.svg /srv/favicon.svg

EXPOSE 8000

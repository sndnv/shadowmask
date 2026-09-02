FROM nginx:stable-alpine

LABEL org.opencontainers.image.description="Web-based user interface for the Shadowmask media server"

RUN mkdir -p /opt/shadowmask-web-ui/templates

COPY build/web /usr/share/nginx/html
COPY deployment/production/.env.template /opt/shadowmask-web-ui/templates
COPY deployment/production/nginx.template /opt/shadowmask-web-ui/templates
COPY deployment/production/entrypoint.sh /opt/shadowmask-web-ui

RUN chmod +x /opt/shadowmask-web-ui/entrypoint.sh

ENTRYPOINT ["/opt/shadowmask-web-ui/entrypoint.sh"]

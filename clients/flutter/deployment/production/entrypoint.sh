#!/usr/bin/env sh

require() {
    VALUE=$1
    MESSAGE=$2

    if [ "${VALUE}" = "" ]
    then
      echo "${MESSAGE}"
      exit 1
    fi
}

export SHADOWMASK_API_BASE="${SHADOWMASK_API_BASE}"
export NGINX_SERVER_NAME="${NGINX_SERVER_NAME:-localhost}"
export NGINX_SERVER_PORT="${NGINX_SERVER_PORT:-80}"

require "${SHADOWMASK_API_BASE}" "<API base URL> (SHADOWMASK_API_BASE) is required"

if echo "${NGINX_SERVER_PORT}" | grep -Eq "ssl$"
then
  require "${NGINX_SERVER_SSL_CERTIFICATE}" "<nginx server SSL certificate> is required"
  require "${NGINX_SERVER_SSL_CERTIFICATE_KEY}" "<nginx server SSL certificate key> is required"
  require "${NGINX_SERVER_SSL_PROTOCOLS}" "<nginx server SSL protocols> is required"
  require "${NGINX_SERVER_SSL_CIPHERS}" "<nginx server SSL ciphers> is required"

  export NGINX_SERVER_SSL_CERTIFICATE_LINE="ssl_certificate ${NGINX_SERVER_SSL_CERTIFICATE};"
  export NGINX_SERVER_SSL_CERTIFICATE_KEY_LINE="ssl_certificate_key ${NGINX_SERVER_SSL_CERTIFICATE_KEY};"
  export NGINX_SERVER_SSL_PROTOCOLS_LINE="ssl_protocols ${NGINX_SERVER_SSL_PROTOCOLS};"
  export NGINX_SERVER_SSL_CIPHERS_LINE="ssl_ciphers ${NGINX_SERVER_SSL_CIPHERS};"
else
  export NGINX_SERVER_SSL_CERTIFICATE_LINE=""
  export NGINX_SERVER_SSL_CERTIFICATE_KEY_LINE=""
  export NGINX_SERVER_SSL_PROTOCOLS_LINE=""
  export NGINX_SERVER_SSL_CIPHERS_LINE=""
fi

envsubst \$SHADOWMASK_API_BASE < /opt/shadowmask-web-ui/templates/.env.template > /usr/share/nginx/html/assets/.env
envsubst \$NGINX_SERVER_NAME,\$NGINX_SERVER_PORT,\$NGINX_SERVER_SSL_CERTIFICATE_LINE,\$NGINX_SERVER_SSL_CERTIFICATE_KEY_LINE,\$NGINX_SERVER_SSL_PROTOCOLS_LINE,\$NGINX_SERVER_SSL_CIPHERS_LINE < /opt/shadowmask-web-ui/templates/nginx.template > /etc/nginx/nginx.conf

echo "Config("
cat /etc/nginx/nginx.conf
echo ")"

nginx -g 'daemon off;'
